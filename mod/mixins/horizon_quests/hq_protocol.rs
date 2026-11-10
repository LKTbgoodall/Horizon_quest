// The messages between the book (client) and the server half, over Horizon's
// mod messages (`horizon:mod_net`, at most 30 000 bytes and 32 a tick each
// way). Pure: no game types. Phase 0 only carries the ping/pong of spike 3.

/// The channel, `mod_id:name`.
pub const CHANNEL: &str = "horizon_quests:spike";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    /// The book opened and asks the server half whether it is there.
    Ping(u32),
    /// The server half answers the ping of the same number.
    Pong(u32),
}

/// `ping:3`, `pong:3`.
pub fn encode(msg: Msg) -> Vec<u8> {
    match msg {
        Msg::Ping(n) => format!("ping:{n}"),
        Msg::Pong(n) => format!("pong:{n}"),
    }
    .into_bytes()
}

/// The message in `data`, if it is one of ours. Data from a client is the
/// player's own bytes: anything else is None.
pub fn decode(data: &[u8]) -> Option<Msg> {
    let text = std::str::from_utf8(data).ok()?;
    let (kind, number) = text.split_once(':')?;
    let n = number.parse().ok()?;
    match kind {
        "ping" => Some(Msg::Ping(n)),
        "pong" => Some(Msg::Pong(n)),
        _ => None,
    }
}

/// The server half's answer to a client's message, if it needs one.
pub fn answer(msg: Msg) -> Option<Msg> {
    match msg {
        Msg::Ping(n) => Some(Msg::Pong(n)),
        Msg::Pong(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip() {
        for msg in [Msg::Ping(0), Msg::Ping(7), Msg::Pong(u32::MAX)] {
            assert_eq!(decode(&encode(msg)), Some(msg));
        }
        assert_eq!(encode(Msg::Ping(3)), b"ping:3");
    }

    #[test]
    fn junk_is_refused() {
        for junk in [&b""[..], b"ping", b"ping:", b"ping:-1", b"hello:1", b"\xff\xfe", b"pong:1:2"] {
            assert_eq!(decode(junk), None, "{junk:?}");
        }
    }

    #[test]
    fn only_pings_get_an_answer() {
        assert_eq!(answer(Msg::Ping(4)), Some(Msg::Pong(4)));
        assert_eq!(answer(Msg::Pong(4)), None);
    }

    #[test]
    fn the_channel_is_valid() {
        let (id, name) = CHANNEL.split_once(':').unwrap();
        assert_eq!(id, super::super::hq_meta::ID);
        assert!((1..=64).contains(&name.len()));
    }
}
