// What the book (client) and the server half say to each other over
// Horizon's mod messages, which carry at most 30 000 bytes each and 32 a
// tick each way. A message is JSON, cut into frames small enough to send; a
// `Sender` paces the frames tick by tick and a `Receiver` puts them back
// together. The server holds the truth: the book only asks, and shows what
// it is sent. Pure: no game types.
//
// A frame is `transfer (u32) | index (u16) | count (u16) | bytes`, little
// endian: part `index` of the `count` parts of message number `transfer`.
use super::hq_edit::Edit;
use super::hq_model::{Progress, QuestFile};
use super::hq_progress::QuestEvent;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// The channel, `mod_id:name`.
pub const CHANNEL: &str = "horizon_quests:book";
/// The version of these messages. A book and a server of different versions
/// do not talk.
pub const PROTOCOL: u32 = 4;

/// The most bytes one mod message carries.
pub const MAX_FRAME: usize = 30_000;
const HEADER: usize = 8;
const MAX_BYTES: usize = MAX_FRAME - HEADER;
/// The frames a `Sender` lets out in one tick: half of what Horizon allows,
/// so other traffic of the mod never hits the limit.
pub const FRAMES_PER_TICK: usize = 16;
/// The most parts the book accepts for one message from the server (about
/// 15 MB), and the server from a book: requests are always one small frame.
pub const CLIENT_MAX_PARTS: u16 = 512;
pub const SERVER_MAX_PARTS: u16 = 1;

/// What the book sends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToServer {
    /// First message on joining: the server answers `Welcome`, then the
    /// book and the player's progress, or `Incompatible`.
    Hello { protocol: u32, version: String },
    /// Tick checkmark task `task`.
    Check { task: String },
    /// Hand in the items of task `task`.
    Submit { task: String },
    /// Claim reward `reward`; `choice` is the table entry picked, for a
    /// choice reward.
    Claim {
        reward: String,
        #[serde(default)]
        choice: Option<u32>,
    },
    ClaimAll,
    /// An operator's change to the pack, from the book's edit mode.
    Edit { edit: Edit },
    /// Copy the pack in use to the modpack's folder, for every world that
    /// has no quests of its own.
    SaveToModpack,
    /// Put this world's own quests aside: the world uses the modpack's (or
    /// the data packs', or the built-in ones) again.
    UseModpack,
}

/// What the server sends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToClient {
    Welcome { protocol: u32, version: String },
    /// The server speaks another protocol: the book says so and stays shut.
    Incompatible { protocol: u32, version: String },
    /// The quest pack, on joining and after a reload.
    Book { file: QuestFile },
    /// The progress of the player's team, whole, and the name claims are
    /// recorded under for this player.
    Progress { player: String, progress: Progress },
    /// A line for the book to show: why a request was refused.
    Notice { text: String },
    /// A toast for the player's screen: a quest completed, a reward. `icon`
    /// is an item id.
    Toast {
        title: String,
        text: String,
        #[serde(default)]
        icon: Option<String>,
    },
    /// The player's party, for the book's header: on joining and whenever
    /// it changes. None: they play on their own.
    Party { name: Option<String> },
    /// Whether this player may edit the quests (an operator), and where the
    /// pack in use comes from, for the book's edit mode.
    Editor { allowed: bool, source: String },
}

/// The server's answer to a book's `Hello`.
pub fn greet(protocol: u32, version: &str) -> ToClient {
    let version = version.to_owned();
    if protocol == PROTOCOL {
        ToClient::Welcome { protocol: PROTOCOL, version }
    } else {
        ToClient::Incompatible { protocol: PROTOCOL, version }
    }
}

/// The engine event a book's request stands for. None for `Hello`, and for
/// `Submit`: the glue first takes the items (`hq_progress::submit_request`).
pub fn request_event(message: &ToServer) -> Option<QuestEvent> {
    match message {
        ToServer::Check { task } => Some(QuestEvent::Check { task: task.clone() }),
        ToServer::Claim { reward, choice } => Some(QuestEvent::Claim { reward: reward.clone(), choice: *choice }),
        ToServer::ClaimAll => Some(QuestEvent::ClaimAll),
        ToServer::Hello { .. }
        | ToServer::Submit { .. }
        | ToServer::Edit { .. }
        | ToServer::SaveToModpack
        | ToServer::UseModpack => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetError {
    /// Not even a frame header, or more than a mod message carries.
    BadSize,
    /// A part that does not follow the one before.
    OutOfOrder,
    /// A message in more parts than this side accepts.
    TooManyParts,
    /// The bytes are not a message this side knows.
    NotAMessage,
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NetError::BadSize => "a frame of the wrong size",
            NetError::OutOfOrder => "a frame out of order",
            NetError::TooManyParts => "a message in too many parts",
            NetError::NotAMessage => "not a message",
        })
    }
}

/// `bytes` as the frames of message number `transfer`. None when it would
/// take more parts than a frame can count.
pub fn frames(transfer: u32, bytes: &[u8]) -> Option<Vec<Vec<u8>>> {
    let count = part_count(bytes.len())?;
    let mut parts = bytes.chunks(MAX_BYTES);
    Some(
        (0..count)
            .map(|index| {
                let part = parts.next().unwrap_or(&[]);
                let mut frame = Vec::with_capacity(HEADER + part.len());
                frame.extend_from_slice(&transfer.to_le_bytes());
                frame.extend_from_slice(&index.to_le_bytes());
                frame.extend_from_slice(&count.to_le_bytes());
                frame.extend_from_slice(part);
                frame
            })
            .collect(),
    )
}

/// How many frames a message of `len` bytes takes (an empty one takes one).
fn part_count(len: usize) -> Option<u16> {
    u16::try_from(len.div_ceil(MAX_BYTES).max(1)).ok()
}

/// One side's outgoing messages to one peer: `send` queues, `tick` hands
/// over what may leave this tick.
#[derive(Clone, Debug, Default)]
pub struct Sender {
    next_transfer: u32,
    queue: VecDeque<Vec<u8>>,
}

impl Sender {
    /// Queues `message`. Err when it cannot be written or is too big.
    pub fn send<T: Serialize>(&mut self, message: &T) -> Result<(), String> {
        let bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
        let frames = frames(self.next_transfer, &bytes).ok_or("the message is too big to send")?;
        self.next_transfer = self.next_transfer.wrapping_add(1);
        self.queue.extend(frames);
        Ok(())
    }

    /// The frames to send this tick, in order.
    pub fn tick(&mut self) -> Vec<Vec<u8>> {
        let count = self.queue.len().min(FRAMES_PER_TICK);
        self.queue.drain(..count).collect()
    }

    /// Puts back frames `tick` handed over that could not leave, to try
    /// again next tick.
    pub fn put_back(&mut self, frames: Vec<Vec<u8>>) {
        for frame in frames.into_iter().rev() {
            self.queue.push_front(frame);
        }
    }

    /// The frames still waiting.
    pub fn waiting(&self) -> usize {
        self.queue.len()
    }

}

/// One side's incoming frames from one peer, put back together. Frames come
/// from the network: nothing in them is trusted.
#[derive(Clone, Debug)]
pub struct Receiver {
    max_parts: u16,
    /// The message being received: its number, how many parts it has and
    /// how many came, and their bytes so far.
    transfer: u32,
    count: u16,
    next: u16,
    bytes: Vec<u8>,
}

impl Receiver {
    pub fn new(max_parts: u16) -> Self {
        Self { max_parts, transfer: 0, count: 0, next: 0, bytes: Vec::new() }
    }

    /// Takes one frame. Some(message) when it was the last part of one.
    /// After an error the message under way is dropped.
    pub fn push<T: DeserializeOwned>(&mut self, frame: &[u8]) -> Result<Option<T>, NetError> {
        let whole = self.push_bytes(frame).inspect_err(|_| self.reset())?;
        match whole {
            Some(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|_| NetError::NotAMessage),
            None => Ok(None),
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.next = 0;
        self.bytes = Vec::new();
    }

    fn push_bytes(&mut self, frame: &[u8]) -> Result<Option<Vec<u8>>, NetError> {
        if frame.len() < HEADER || frame.len() > MAX_FRAME {
            return Err(NetError::BadSize);
        }
        let transfer = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        let index = u16::from_le_bytes([frame[4], frame[5]]);
        let count = u16::from_le_bytes([frame[6], frame[7]]);
        if count == 0 || index >= count {
            return Err(NetError::OutOfOrder);
        }
        if count > self.max_parts {
            return Err(NetError::TooManyParts);
        }
        if index == 0 {
            // A new message; one left unfinished is dropped.
            self.reset();
            self.transfer = transfer;
            self.count = count;
        } else if transfer != self.transfer || count != self.count || index != self.next {
            return Err(NetError::OutOfOrder);
        }
        self.bytes.extend_from_slice(&frame[HEADER..]);
        self.next = index + 1;
        if self.next < self.count {
            return Ok(None);
        }
        let bytes = std::mem::take(&mut self.bytes);
        self.reset();
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends `message` through a sender and a receiver, tick after tick.
    fn carry<T: Serialize + DeserializeOwned>(message: &T, max_parts: u16) -> (Result<Option<T>, NetError>, usize) {
        let mut sender = Sender::default();
        sender.send(message).unwrap();
        let mut receiver = Receiver::new(max_parts);
        let (mut last, mut ticks) = (Ok(None), 0);
        while sender.waiting() > 0 {
            ticks += 1;
            for frame in sender.tick() {
                assert!(frame.len() <= MAX_FRAME);
                last = receiver.push(&frame);
                if last.is_err() {
                    return (last, ticks);
                }
            }
        }
        (last, ticks)
    }

    #[test]
    fn requests_round_trip_in_one_frame() {
        for message in [
            ToServer::Hello { protocol: PROTOCOL, version: "0.1.0".into() },
            ToServer::Check { task: "a/q/t".into() },
            ToServer::Submit { task: "a/q/t".into() },
            ToServer::Claim { reward: "a/q/r".into(), choice: None },
            ToServer::Claim { reward: "a/q/r".into(), choice: Some(2) },
            ToServer::ClaimAll,
        ] {
            assert_eq!(carry(&message, SERVER_MAX_PARTS), (Ok(Some(message.clone())), 1));
        }
        let text = String::from_utf8(serde_json::to_vec(&ToServer::Check { task: "a/q/t".into() }).unwrap()).unwrap();
        assert_eq!(text, r#"{"type":"check","task":"a/q/t"}"#);
    }

    #[test]
    fn a_big_book_travels_in_parts_over_several_ticks() {
        // A pack far bigger than one frame: 40 chapters of long text.
        let mut file = QuestFile::default();
        for c in 0..40 {
            let mut chapter = super::super::hq_model::Chapter { id: format!("chapter_{c}"), ..Default::default() };
            for q in 0..50 {
                chapter.quests.push(super::super::hq_model::Quest {
                    id: format!("quest_{q}"),
                    description: vec!["A long paragraph of quest text. ".repeat(10)],
                    ..Default::default()
                });
            }
            file.chapters.push(chapter);
        }
        let message = ToClient::Book { file };
        let size = serde_json::to_vec(&message).unwrap().len();
        assert!(size > 20 * MAX_FRAME, "{size}");

        let (received, ticks) = carry(&message, CLIENT_MAX_PARTS);
        assert_eq!(received, Ok(Some(message.clone())));
        assert_eq!(ticks, size.div_ceil(MAX_BYTES).div_ceil(FRAMES_PER_TICK));
        // The server never accepts that much from a book.
        assert_eq!(carry(&message, SERVER_MAX_PARTS).0, Err(NetError::TooManyParts));
    }

    #[test]
    fn progress_and_notices_round_trip() {
        let mut progress = Progress::default();
        progress.tasks.insert("a/q/t".into(), 3);
        progress.claimed.entry("a/q/r".into()).or_default().insert("steve".into());
        for message in [
            ToClient::Progress { player: "steve".into(), progress },
            ToClient::Notice { text: "There is nothing to claim.".into() },
            ToClient::Toast { title: "Quest completed".into(), text: "Timber".into(), icon: Some("minecraft:oak_log".into()) },
            ToClient::Party { name: Some("Miners".into()) },
            ToClient::Party { name: None },
            greet(PROTOCOL, "0.1.0"),
        ] {
            assert_eq!(carry(&message, CLIENT_MAX_PARTS).0, Ok(Some(message.clone())));
        }
    }

    #[test]
    fn frames_are_cut_at_the_limit() {
        let count = |len: usize| frames(7, &vec![1; len]).unwrap().len();
        assert_eq!((count(0), count(1), count(MAX_BYTES), count(MAX_BYTES + 1)), (1, 1, 1, 2));
        let cut = frames(7, &vec![1; MAX_BYTES + 5]).unwrap();
        assert_eq!((cut[0].len(), cut[1].len()), (MAX_FRAME, HEADER + 5));
        assert_eq!(cut[1][..HEADER], [7, 0, 0, 0, 1, 0, 2, 0]);
        // A frame counts its parts on 16 bits.
        assert_eq!(part_count(MAX_BYTES * 65_535), Some(u16::MAX));
        assert_eq!(part_count(MAX_BYTES * 65_535 + 1), None);
    }

    #[test]
    fn greetings() {
        assert_eq!(greet(PROTOCOL, "1.0"), ToClient::Welcome { protocol: PROTOCOL, version: "1.0".into() });
        assert_eq!(greet(PROTOCOL + 1, "1.0"), ToClient::Incompatible { protocol: PROTOCOL, version: "1.0".into() });
    }

    #[test]
    fn requests_become_events() {
        let check = ToServer::Check { task: "a/q/t".into() };
        assert_eq!(request_event(&check), Some(QuestEvent::Check { task: "a/q/t".into() }));
        let claim = ToServer::Claim { reward: "a/q/r".into(), choice: Some(1) };
        assert_eq!(request_event(&claim), Some(QuestEvent::Claim { reward: "a/q/r".into(), choice: Some(1) }));
        // A claim that says nothing of a choice has none.
        let plain: ToServer = serde_json::from_str(r#"{"type":"claim","reward":"a/q/r"}"#).unwrap();
        assert_eq!(plain, ToServer::Claim { reward: "a/q/r".into(), choice: None });
        assert_eq!(request_event(&ToServer::ClaimAll), Some(QuestEvent::ClaimAll));
        assert_eq!(request_event(&ToServer::Submit { task: "a/q/t".into() }), None);
        assert_eq!(request_event(&ToServer::Hello { protocol: 1, version: String::new() }), None);
    }

    #[test]
    fn junk_frames_are_refused_and_do_not_stick() {
        let mut receiver = Receiver::new(4);
        let push = |receiver: &mut Receiver, frame: &[u8]| receiver.push::<ToServer>(frame);
        assert_eq!(push(&mut receiver, b""), Err(NetError::BadSize));
        assert_eq!(push(&mut receiver, b"ping:1"), Err(NetError::BadSize));
        assert_eq!(push(&mut receiver, &vec![0; MAX_FRAME + 1]), Err(NetError::BadSize));
        // count 0, index past count, too many parts, a part with no start.
        assert_eq!(push(&mut receiver, &[0, 0, 0, 0, 0, 0, 0, 0]), Err(NetError::OutOfOrder));
        assert_eq!(push(&mut receiver, &[0, 0, 0, 0, 2, 0, 2, 0]), Err(NetError::OutOfOrder));
        assert_eq!(push(&mut receiver, &[0, 0, 0, 0, 0, 0, 5, 0]), Err(NetError::TooManyParts));
        assert_eq!(push(&mut receiver, &[0, 0, 0, 0, 1, 0, 2, 0]), Err(NetError::OutOfOrder));
        // One whole frame that is not a message.
        assert_eq!(push(&mut receiver, &[0, 0, 0, 0, 0, 0, 1, 0, b'?']), Err(NetError::NotAMessage));
        let unknown = frames(3, br#"{"type":"dance"}"#).unwrap();
        assert_eq!(push(&mut receiver, &unknown[0]), Err(NetError::NotAMessage));

        // A message cut short by another, then a good one: the good one comes through.
        let parts = frames(1, &vec![b' '; MAX_BYTES + 1]).unwrap();
        assert_eq!(push(&mut receiver, &parts[0]), Ok(None));
        let other = frames(2, br#"{"type":"claim_all"}"#).unwrap();
        assert_eq!(push(&mut receiver, &other[0]), Ok(Some(ToServer::ClaimAll)));
        // The first message's second part no longer follows anything.
        assert_eq!(push(&mut receiver, &parts[1]), Err(NetError::OutOfOrder));
        assert_eq!(push(&mut receiver, &other[0]), Ok(Some(ToServer::ClaimAll)));
    }

    #[test]
    fn a_sender_paces_and_takes_frames_back() {
        let mut sender = Sender::default();
        for _ in 0..FRAMES_PER_TICK + 3 {
            sender.send(&ToServer::ClaimAll).unwrap();
        }
        assert_eq!(sender.waiting(), FRAMES_PER_TICK + 3);
        let first = sender.tick();
        assert_eq!((first.len(), sender.waiting()), (FRAMES_PER_TICK, 3));
        // Each message has its own number.
        assert_eq!((first[0][0], first[1][0], first[2][0]), (0, 1, 2));

        // The last two could not leave: they go first next tick.
        sender.put_back(first[FRAMES_PER_TICK - 2..].to_vec());
        let second = sender.tick();
        assert_eq!(second.len(), 5);
        assert_eq!(second[..2], first[FRAMES_PER_TICK - 2..]);
        assert!(sender.tick().is_empty());
        assert_eq!(sender.waiting(), 0);
    }

    #[test]
    fn the_channel_is_valid() {
        let (id, name) = CHANNEL.split_once(':').unwrap();
        assert_eq!(id, super::super::hq_meta::ID);
        assert!((1..=64).contains(&name.len()));
        assert_eq!(NetError::OutOfOrder.to_string(), "a frame out of order");
    }
}
