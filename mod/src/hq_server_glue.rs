// The server half (a single player's server thread, a LAN host, a dedicated
// server). It keeps a `Session` (hq_session) while a world is open: hooks
// only record what happened (a kill, a craft, a level, an advancement), and
// the server tick looks at inventories and at where players stand, runs the
// engine, gives rewards, tells the players, syncs their books over the
// channel "horizon_quests:book" (hq_net) and saves.
// Progress is kept per team (hq_teams): a lone player's file is their UUID's,
// a party's is its id's, and teams.json says who is in which party.
// The state is behind a Mutex. The tick holds it while it reads
// (inventories, item tags, the players' places), which call no hook back; it
// is released before anything that changes the game (rewards, chat, sends,
// commands).

use super::hq_built_in as built_in;
use super::hq_commands::{self as commands, Command, TeamCommand};
use super::hq_config::{self as config, ServerConfig};
use super::hq_edit::{self as edit, Edit};
use super::hq_logic::Book;
use super::hq_meta as meta;
use super::hq_model::{Progress, QuestFile, RewardKind};
use super::hq_net::{self as net, ToClient, ToServer};
use super::hq_net_glue;
use super::hq_progress::{self as engine, Effect, QuestEvent, Wanted};
use super::hq_session::{self as session, Rules, Session, Verb};
use super::hq_store as store;
use super::hq_validate::{self as validate, Loaded};
use mod_api::{
    Command as GameCommand, CommandArgumentValue, CommandInvocation, CommandSource, Config, ConfigFile, ConfigKind,
    FileToIdConverter, Host, Identifier, ItemStack, PayloadBytes, PlayerRef, ReloadApply, ReloadResourceError,
    ServerHost, ServerReloadContext, ServerReloadListener, ServerReloadListeners, ServerTickPhase, Text,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// The quest files the data packs hold (`data/<namespace>/horizon_quests/`),
/// as the last data load handed them over: (`<namespace>:<path>`, JSON).
static DATA: Mutex<Vec<(String, serde_json::Value)>> = Mutex::new(Vec::new());

/// The id of our reload listener.
const LISTENER: &str = "horizon_quests:quests";

/// The advancements the data load found (id, icon item), for the editor's
/// picker, and the id of the listener that reads them.
static ADVANCEMENTS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
const ADVANCEMENT_LISTENER: &str = "horizon_quests:advancements";

/// What a book is told about edit mode: whether this player may edit, where
/// the pack comes from, and the advancements an editor may pick.
fn editor_message(allowed: bool, source: &str) -> ToClient {
    let advancements = if allowed { ADVANCEMENTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone() } else { Vec::new() };
    ToClient::Editor { allowed, source: source.to_owned(), advancements }
}

/// The open world's quest state; None while no world is open.
static STATE: Mutex<Option<State>> = Mutex::new(None);

/// The server's settings (hq_config), as its config file last said. Read
/// before the world opens; the defaults until then.
static CONFIG: Mutex<ServerConfig> = Mutex::new(ServerConfig::DEFAULT);

struct State {
    session: Session,
    /// `<world>/horizon-quests/`; None when the world has no folder.
    dir: Option<PathBuf>,
    /// What `/quests validate` answers for the pack in use.
    report: Vec<String>,
    /// Where the pack in use comes from (`hq_edit::SOURCE_*`).
    source: &'static str,
    /// The modpack's pack folder, `config/horizon_quests/quests/`, used by
    /// every world without quests of its own.
    modpack: Option<PathBuf>,
    /// Whether each player's book was told it may edit (`ToClient::Editor`).
    editors: HashMap<u128, bool>,
    /// Players `/quests edit` gave the pencil to, whatever their level.
    granted: HashSet<u128>,
    /// Effects decided outside the tick (an admin command), carried out on
    /// the next one.
    pending: Vec<(u128, Effect)>,
    /// Outgoing net senders to online players (hq_net).
    senders: HashMap<u128, net::Sender>,
    /// Incoming net receivers from online players (hq_net).
    receivers: HashMap<u128, net::Receiver>,
    /// Players whose answer to a `Hello` (the whole pack) is still being
    /// sent: another `Hello` from them is ignored until it has left.
    greeting: HashSet<u128>,
    /// What players took out of a crafting grid or a furnace since the last
    /// tick (player, item, count): the tick matches it against craft tasks,
    /// where the item tags can be read.
    crafts: Vec<(u128, String, u32)>,
    /// Players to give a quest book on the next tick (their first join,
    /// `/quests book`).
    books: Vec<u128>,
}

/// How many problems a report lists in chat.
const REPORT_LIMIT: usize = 8;

fn state() -> MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn server_config() -> ServerConfig {
    *CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The pack: `<world>/horizon-quests/quests/` when the world has one, else
/// the modpack's (`config/horizon_quests/quests/`), else the chapters of the
/// world's data packs (`data/<namespace>/horizon_quests/`), else the example
/// pack compiled into the mod (hq_built_in), unless the server's config
/// turned it off. With where it came from, for the log and for the book
/// (`hq_edit::SOURCE_*`).
fn load_pack(dir: Option<&Path>, modpack: Option<&Path>) -> (Loaded, &'static str, &'static str) {
    if let Some(quests) = dir.map(|d| d.join(store::PACK_FOLDER))
        && quests.join("quests.json").is_file()
    {
        return (validate::load_dir(&quests), "the world's quests folder", edit::SOURCE_WORLD);
    }
    if let Some(quests) = modpack.filter(|m| m.join("quests.json").is_file()) {
        return (validate::load_dir(quests), "the modpack's quests folder (config/horizon_quests/quests/)", edit::SOURCE_MODPACK);
    }
    let data = validate::load_data(&DATA.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
    if let Some(loaded) = data {
        return (loaded, "the data packs (data/<namespace>/horizon_quests/)", edit::SOURCE_DATA_PACKS);
    }
    if !server_config().example_chapter {
        let empty = validate::load_pack(None, &[], &[]);
        return (empty, "nowhere: the example chapters are turned off (example_chapter = false)", edit::SOURCE_NONE);
    }
    (validate::load_pack(Some(built_in::SETTINGS), built_in::CHAPTERS, built_in::TABLES), "the built-in example pack", edit::SOURCE_BUILT_IN)
}

/// What the server's settings ask of the session.
fn rules(config: &ServerConfig) -> Rules {
    Rules { parties: config.parties, party_size: config.party_size, detection_delay: config.detection_delay }
}

/// A new state for the open world, its pack loaded and reported in the log.
fn open(host: &dyn ServerHost) -> State {
    let dir = host.world_dir().map(|w| w.join(store::FOLDER));
    let modpack = Some(host.instance_dir().join("config").join(meta::ID).join(store::PACK_FOLDER));
    let (loaded, from, source) = load_pack(dir.as_deref(), modpack.as_deref());
    let report = commands::report(&loaded, REPORT_LIMIT);
    log::info!("{}: quests from {from}: {}", meta::NAME, report.join(" | "));
    let mut session = Session::new(Book::new(loaded.file));
    let parties = dir.as_deref().map_or_else(|| Ok(Default::default()), store::load_teams);
    if let Err(e) = &parties {
        log::error!("{}: the parties are not readable, so they are off and the file stays as it is: {e}", meta::NAME);
    }
    session.set_parties(parties);
    session.set_rules(rules(&server_config()));
    State {
        session,
        dir,
        report,
        source,
        modpack,
        editors: HashMap::new(),
        granted: HashSet::new(),
        pending: Vec::new(),
        senders: HashMap::new(),
        receivers: HashMap::new(),
        greeting: HashSet::new(),
        crafts: Vec::new(),
        books: Vec::new(),
    }
}

fn player(host: &dyn ServerHost, uuid: u128) -> Option<PlayerRef> {
    host.player(uuid)
}

fn tell(host: &mut dyn ServerHost, uuid: u128, line: &str) {
    if let Some(player) = player(host, uuid) {
        let _ = player.tell(host, Text::literal(format!("[{}] {line}", meta::NAME)));
    }
}

/// The server opened its worlds.
pub(super) fn world_opened(host: &mut dyn ServerHost) {
    let fresh = open(host);
    *state() = Some(fresh);
}

/// The server stopped: what changed is saved, and the next world's data load
/// hands its own files over.
pub(super) fn world_closed() {
    if let Some(mut s) = state().take() {
        save(&mut s);
    }
    DATA.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
}

/// Writes every changed progress, and the parties when they changed.
fn save(s: &mut State) {
    let Some(dir) = s.dir.clone() else {
        return;
    };
    for (team, progress) in s.session.take_dirty() {
        if let Err(e) = store::save_progress(&dir, &team, &progress) {
            log::error!("{}: could not save the progress of {team}: {e}", meta::NAME);
        }
    }
    if let Some(parties) = s.session.take_parties()
        && let Err(e) = store::save_teams(&dir, &parties)
    {
        log::error!("{}: could not save the parties: {e}", meta::NAME);
    }
}

/// A team's progress as its file says, for a team none of whose members is on.
fn load_team(dir: Option<&Path>, team: &str) -> Result<Progress, String> {
    dir.map_or_else(|| Ok(Progress::default()), |dir| store::load_progress(dir, team))
}

pub(super) fn logged_in(host: &mut dyn ServerHost, player: u128, name: &str) {
    let mut guard = state();
    if guard.is_none() {
        *guard = Some(open(host));
    }
    let Some(s) = guard.as_mut() else {
        return;
    };
    // Their team's progress: in play already when a member is on.
    let team = s.session.team_of(player);
    let progress = if s.session.team_loaded(&team) { Ok(Progress::default()) } else { load_team(s.dir.as_deref(), &team) };
    if let Err(e) = &progress {
        log::error!("{}: {name}'s progress ({team}) is not readable, it will not be saved: {e}", meta::NAME);
    }
    s.session.join(player, name, progress);
}

pub(super) fn logged_out(player: u128) {
    if let Some(s) = state().as_mut() {
        s.senders.remove(&player);
        s.receivers.remove(&player);
        s.editors.remove(&player);
        s.greeting.remove(&player);
        if let Some((team, progress)) = s.session.leave(player)
            && let Some(dir) = &s.dir
            && let Err(e) = store::save_progress(dir, &team, &progress)
        {
            log::error!("{}: could not save the progress of {team}: {e}", meta::NAME);
        }
    }
}

/// The player's inventory changed: look at it next tick.
pub(super) fn poke(player: u128) {
    if let Some(s) = state().as_mut() {
        s.session.poke(player);
    }
}

pub(super) fn mob_killed(player: u128, kind: &str) {
    if let Some(s) = state().as_mut() {
        s.session.push(player, QuestEvent::Killed { entity: session::entity_id(kind) });
    }
}

/// A player took `count` of `item` out of a crafting grid or a furnace:
/// craft tasks count it on the next tick, and their inventory is looked at.
pub(super) fn crafted(player: u128, item: &str, count: u32) {
    if let Some(s) = state().as_mut() {
        s.session.poke(player);
        s.crafts.push((player, item.to_owned(), count));
    }
}

/// A player's experience level changed to `level`.
pub(super) fn level_changed(player: u128, level: i32) {
    if let Some(s) = state().as_mut() {
        s.session.set_level(player, level);
    }
}

/// A player completed an advancement.
pub(super) fn advancement_done(player: u128, advancement: &str) {
    if let Some(s) = state().as_mut() {
        s.session.advancement_done(player, advancement);
    }
}

/// What a tick decided, carried out once the lock is released.
struct Action {
    uuid: u128,
    name: String,
    line: Option<String>,
    /// A reward to give: what the pack calls it, and what it is.
    reward: Option<(Option<String>, RewardKind)>,
}

/// The end of every server tick: inventories, engine, effects, saving.
pub(super) fn server_tick(host: &mut dyn ServerHost, phase: ServerTickPhase) {
    if !matches!(phase, ServerTickPhase::ServerTickEnd { .. }) {
        return;
    }
    let config = server_config();
    let (actions, outgoing, probes) = {
        let mut guard = state();
        let Some(s) = guard.as_mut() else {
            return;
        };
        for (uuid, item, count) in std::mem::take(&mut s.crafts) {
            let tags = |tag: &str, item: &str| in_tag(host, tag, item);
            s.session.crafted(uuid, &item, count, &tags);
        }
        let mut probes = Vec::new();
        for uuid in s.session.advance() {
            look_at_inventory(host, &mut s.session, uuid);
            if config.quest_book && s.session.take_book(uuid) {
                s.books.push(uuid);
            }
            let probe = s.session.take_probe(uuid);
            if probe != engine::Probe::default()
                && let Some(player) = s.session.player(uuid)
                && session::safe_name(&player.name)
            {
                probes.push((uuid, player.name.clone(), probe));
            }
            if s.session.observe(uuid)
                && let Some((dimension, position)) = place(host, uuid)
            {
                s.session.at(uuid, &dimension, position);
            }
        }
        let mut effects = std::mem::take(&mut s.pending);
        effects.extend(s.session.process(session::now()));

        // Changed progress goes to the books, under the id claims are kept
        // under (tracked apart from saving: an unsaveable one is sent once).
        for (uuid, id, progress) in s.session.take_unsynced() {
            if let Some(sender) = s.senders.get_mut(&uuid) {
                let _ = sender.send(&ToClient::Progress { player: id, progress });
            }
        }
        for (uuid, name) in s.session.take_party_changes() {
            if let Some(sender) = s.senders.get_mut(&uuid) {
                let _ = sender.send(&ToClient::Party { name });
            }
        }

        let book = s.session.book();
        let mut actions: Vec<Action> = effects
            .into_iter()
            .map(|(uuid, effect)| Action {
                uuid,
                name: s.session.player(uuid).map(|p| p.name.clone()).unwrap_or_default(),
                line: match effect {
                    Effect::QuestCompleted { .. } | Effect::ChapterCompleted { .. } if !config.completion_messages => None,
                    _ => commands::announce(book, &effect),
                },
                reward: match effect {
                    Effect::Reward { title, kind, .. } => Some((title, kind)),
                    _ => None,
                },
            })
            .collect();
        for uuid in std::mem::take(&mut s.books) {
            actions.push(Action {
                uuid,
                name: s.session.player(uuid).map(|p| p.name.clone()).unwrap_or_default(),
                line: Some(format!("Here is your {}: right-click it to open your quests.", meta::BOOK_NAME)),
                reward: Some((None, RewardKind::Item { item: meta::BOOK_ITEM.to_owned(), count: 1, max_count: None })),
            });
        }
        save(s);

        let mut outgoing = Vec::new();
        for (&uuid, sender) in &mut s.senders {
            let frames = sender.tick();
            if !frames.is_empty() {
                outgoing.push((uuid, frames));
            }
        }
        let senders = &s.senders;
        s.greeting.retain(|uuid| senders.get(uuid).is_some_and(|sender| sender.waiting() > 0));

        (actions, outgoing, probes)
    };
    for action in actions {
        if let Some(line) = &action.line {
            tell(host, action.uuid, line);
        }
        if let Some((title, reward)) = action.reward {
            give(host, action.uuid, &action.name, title.as_deref(), reward);
        }
    }
    // Levels and advancements the events may have missed: the level as the
    // player's experience says, advancements asked of vanilla's selectors,
    // once the lock is released.
    for (uuid, name, probe) in probes {
        let level = probe.level.and_then(|_| player(host, uuid)?.experience(host)).map(|xp| xp.level);
        let mut asks = |line: Option<String>| line.is_some_and(|line| host.commands().run_command(&line));
        let done: Vec<String> = probe.advancements.into_iter().filter(|a| asks(session::advancement_probe_command(&name, a))).collect();
        if let Some(s) = state().as_mut() {
            if let Some(level) = level {
                s.session.set_level(uuid, level);
            }
            for advancement in done {
                s.session.advancement_done(uuid, &advancement);
            }
        }
    }
    let channel = hq_net_glue::channel();
    for (uuid, frames) in outgoing {
        let Some(to) = player(host, uuid) else {
            continue;
        };
        let mut put_back = Vec::new();
        for frame in frames {
            if to.send_payload(host, &channel, PayloadBytes(frame.clone())).is_err() {
                put_back.push(frame);
            }
        }
        if !put_back.is_empty()
            && let Some(s) = state().as_mut()
            && let Some(sender) = s.senders.get_mut(&uuid)
        {
            sender.put_back(put_back);
        }
    }
}

/// The player's non-empty stacks: (item id, count, plain: no damage and no
/// components).
fn stacks(host: &dyn ServerHost, uuid: u128) -> Option<Vec<(String, u32, bool)>> {
    let slots = player(host, uuid)?.inventory_slots(host)?;
    Some(
        slots
            .into_iter()
            .map(|(_, stack)| (stack.item().id(host), stack.count(), stack.damage() == 0 && stack.components().is_none()))
            .collect(),
    )
}

/// Counts what the player holds for their item tasks.
fn look_at_inventory(host: &dyn ServerHost, session: &mut Session, uuid: u128) {
    let wanted = session.wanted(uuid);
    if wanted.is_empty() {
        return;
    }
    // Logging in, dead, or between two states: the next look will do.
    let Some(held) = stacks(host, uuid) else {
        return;
    };
    let counted: Vec<(&str, u32)> = held.iter().map(|(item, count, _)| (item.as_str(), *count)).collect();
    let tags = |tag: &str, item: &str| in_tag(host, tag, item);
    for event in session::item_events(&wanted, &counted, &tags) {
        session.push(uuid, event);
    }
}

/// Where player `uuid` stands.
fn place(host: &dyn ServerHost, uuid: u128) -> Option<(String, [f64; 3])> {
    let player = player(host, uuid)?;
    let dimension = player.dimension(host)?.to_string();
    let position = player.entity().position(host)?;
    Some((dimension, position))
}

fn in_tag(host: &dyn Host, tag: &str, item: &str) -> bool {
    let tags = host.tags();
    let (Some(item), Some(tag)) = (
        Identifier::parse(item).ok().and_then(|id| host.item_key(&id)),
        Identifier::parse(tag).ok().and_then(|id| tags.key(&id)),
    ) else {
        return false;
    };
    tags.is_in_tag(item, tag).unwrap_or(false)
}

/// Gives a reward; `title` is what the pack calls it. Items that do not fit
/// are dropped at the player's feet, as `/give` does.
fn give(host: &mut dyn ServerHost, uuid: u128, name: &str, title: Option<&str>, reward: RewardKind) {
    let given = match reward {
        RewardKind::Item { item, count, .. } => {
            let stack = Identifier::parse(&item).ok().and_then(|id| host.item_key(&id)).and_then(|key| ItemStack::new(key, count));
            match (stack, player(host, uuid)) {
                (Some(stack), Some(player)) => player.give_stack(host, stack).is_ok(),
                _ => false,
            }
        }
        RewardKind::Xp { points } => player(host, uuid).is_some_and(|player| player.give_experience_points(host, points)),
        RewardKind::XpLevels { levels } => {
            player(host, uuid).is_some_and(|player| player.give_levels(host, i32::try_from(levels).unwrap_or(i32::MAX)))
        }
        RewardKind::Command { command } => {
            session::reward_command(&command, name, uuid).is_some_and(|line| host.commands().run_command(&line))
        }
        RewardKind::Toast { text } => {
            // On the screen of a player whose book has talked to us, else
            // in their chat.
            let (title, text) = session::toast(title, &text);
            let toast = ToClient::Toast { title: title.clone(), text: text.clone(), icon: None };
            let sent = state().as_mut().and_then(|s| s.senders.get_mut(&uuid)).is_some_and(|sender| sender.send(&toast).is_ok());
            if !sent {
                tell(host, uuid, &session::toast_line(&title, &text));
            }
            true
        }
        // The engine draws or asks for the table's entry: what reaches the
        // glue is that entry's kind, never the table.
        RewardKind::Random { .. } | RewardKind::Choice { .. } => false,
    };
    if !given {
        log::warn!("{}: a reward for {name} could not be given", meta::NAME);
        tell(host, uuid, "A reward could not be given. Tell an operator.");
    }
}

/// A frame from a player's client on channel "horizon_quests:book".
pub(super) fn from_client(host: &mut dyn ServerHost, player_id: u128, data: &[u8]) {
    let to_server = {
        let mut guard = state();
        let Some(s) = guard.as_mut() else {
            return;
        };
        let receiver = s.receivers.entry(player_id).or_insert_with(|| net::Receiver::new(net::SERVER_MAX_PARTS));
        match receiver.push::<ToServer>(data) {
            Ok(Some(msg)) => msg,
            Ok(None) => return,
            Err(e) => {
                log::warn!("{}: bad net frame from {player_id}: {e}", meta::NAME);
                return;
            }
        }
    };

    // Whether this player may edit: an operator, or one `/quests edit` let in.
    let level = player(host, player_id).and_then(|p| p.permission_level(host)).unwrap_or(0);
    let may_edit = |s: &State| s.granted.contains(&player_id) || level >= edit::EDIT_LEVEL;
    match to_server {
        ToServer::Hello { protocol, version: _ } => {
            let mut guard = state();
            let Some(s) = guard.as_mut() else {
                return;
            };
            // A book that asks again while the pack is still on its way
            // would only queue it once more.
            if !s.greeting.insert(player_id) {
                return;
            }
            let allowed = may_edit(s);
            let sender = s.senders.entry(player_id).or_default();
            let _ = sender.send(&net::greet(protocol, meta::VERSION));
            if protocol == net::PROTOCOL {
                let _ = sender.send(&ToClient::Book { file: s.session.book().file().clone() });
                if let Some(progress) = s.session.progress(player_id) {
                    let _ = sender.send(&ToClient::Progress { player: session::player_id(player_id), progress: progress.clone() });
                }
                let _ = sender.send(&ToClient::Party { name: s.session.party_of(player_id) });
                let _ = sender.send(&editor_message(allowed, s.source));
                s.editors.insert(player_id, allowed);
            }
        }
        ToServer::Edit { .. } | ToServer::SaveToModpack | ToServer::UseModpack => {
            let mut guard = state();
            let Some(s) = guard.as_mut() else {
                return;
            };
            let line = if !may_edit(s) {
                "Only operators can edit the quests.".to_owned()
            } else {
                match to_server {
                    ToServer::Edit { edit } => apply_edit(s, &edit).err().unwrap_or_default(),
                    ToServer::SaveToModpack => save_to_modpack(s),
                    _ => use_modpack(s),
                }
            };
            if !line.is_empty() {
                let _ = s.senders.entry(player_id).or_default().send(&ToClient::Notice { text: line });
            }
        }
        ToServer::Mark { id, done } => {
            let mut guard = state();
            let Some(s) = guard.as_mut() else {
                return;
            };
            let result = if !may_edit(s) {
                Err("Only operators can mark tasks and rewards.".to_owned())
            } else if s.session.book().find_task(&id).is_some() {
                s.session.mark_task(player_id, &id, done, session::now()).map(|effects| s.pending.extend(effects.into_iter().map(|e| (player_id, e))))
            } else {
                s.session.mark_reward(player_id, &id, done)
            };
            if let Err(line) = result {
                let _ = s.senders.entry(player_id).or_default().send(&ToClient::Notice { text: line });
            }
        }
        ToServer::Check { .. } | ToServer::Claim { .. } | ToServer::ClaimAll => {
            // A request the engine takes as it is, on the next tick.
            if let Some(event) = net::request_event(&to_server)
                && let Some(s) = state().as_mut()
            {
                s.session.push(player_id, event);
            }
        }
        ToServer::Submit { task } => {
            let lines = submit(host, player_id, &task);
            if let Some(line) = lines.first()
                && let Some(s) = state().as_mut()
            {
                let _ = s.senders.entry(player_id).or_default().send(&ToClient::Notice { text: line.clone() });
            }
        }
    }
}

/// An operator's edit: applied to the pack in use, checked, saved as the
/// world's own pack and sent to every book.
fn apply_edit(s: &mut State, change: &Edit) -> Result<(), String> {
    let mut file = s.session.book().file().clone();
    edit::apply(&mut file, change)?;
    let file = edit::check(&file)?;
    let dir = s.dir.as_deref().ok_or_else(|| "This world has no folder to save quests in.".to_owned())?;
    store::save_pack(&dir.join(store::PACK_FOLDER), &file).map_err(|e| {
        log::error!("{}: could not save the edited quests: {e}", meta::NAME);
        "The quests could not be saved: see the log.".to_owned()
    })?;
    s.source = edit::SOURCE_WORLD;
    set_pack(s, file);
    Ok(())
}

/// Puts `file` in use and sends it, and where it comes from, to every book.
fn set_pack(s: &mut State, file: QuestFile) {
    let loaded = Loaded { file: file.clone(), problems: Vec::new() };
    s.report = commands::report(&loaded, REPORT_LIMIT);
    s.session.set_book(Book::new(file.clone()));
    for (player, sender) in &mut s.senders {
        let _ = sender.send(&ToClient::Book { file: file.clone() });
        let allowed = s.editors.get(player).copied().unwrap_or(false);
        let _ = sender.send(&editor_message(allowed, s.source));
    }
}

/// Copies the pack in use to the modpack's folder: worlds without quests
/// of their own use it from then on.
fn save_to_modpack(s: &State) -> String {
    let Some(folder) = &s.modpack else {
        return "There is no config folder to save the modpack's quests in.".to_owned();
    };
    match store::save_pack(folder, s.session.book().file()) {
        Ok(()) => "Saved for the modpack (config/horizon_quests/quests/): every world without quests of its own uses them.".to_owned(),
        Err(e) => {
            log::error!("{}: could not save the modpack's quests: {e}", meta::NAME);
            "The modpack's quests could not be saved: see the log.".to_owned()
        }
    }
}

/// Puts the world's own quests aside (renamed, never deleted) and loads
/// the modpack's, or whatever comes next.
fn use_modpack(s: &mut State) -> String {
    let Some(own) = s.dir.as_ref().map(|d| d.join(store::PACK_FOLDER)).filter(|own| own.is_dir()) else {
        return "This world has no quests of its own.".to_owned();
    };
    let aside = own.with_file_name(format!("{}-aside-{}", store::PACK_FOLDER, session::now()));
    if let Err(e) = std::fs::rename(&own, &aside) {
        log::error!("{}: could not put {} aside: {e}", meta::NAME, own.display());
        return "This world's quests could not be put aside: see the log.".to_owned();
    }
    let (loaded, from, source) = load_pack(s.dir.as_deref(), s.modpack.as_deref());
    s.source = source;
    set_pack(s, loaded.file);
    format!("Now using {from}. This world's quests were kept in {}.", aside.file_name().unwrap_or_default().to_string_lossy())
}

/// The mod's server config file: `config/horizon_quests-server.toml`, or the
/// world's own copy in `serverconfig/`.
pub(super) fn config_file() -> Config {
    Config::new(meta::ID, ConfigKind::Server, config::DEFAULT_TEXT)
}

/// A config file was read (`reloaded`: it changed on disk). Ours sets the
/// server's settings; a change made while a world is open applies at once,
/// as `/quests reload` does.
pub(super) fn config_read(file: &ConfigFile, reloaded: bool) {
    if file.mod_id != meta::ID || file.kind != ConfigKind::Server {
        return;
    }
    let (read, problems) = config::parse(&file.text);
    for problem in &problems {
        log::warn!("{}: {}: {problem}", meta::NAME, file.file);
    }
    let before = std::mem::replace(&mut *CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()), read);
    if let Some(s) = state().as_mut() {
        s.session.set_rules(rules(&read));
    }
    if reloaded && before != read {
        let lines = reload();
        log::info!("{}: {} changed: {}", meta::NAME, file.file, lines.join(" | "));
    }
}

/// The server stopped: the next world reads its config again.
pub(super) fn config_dropped(file: &ConfigFile) {
    if file.mod_id == meta::ID && file.kind == ConfigKind::Server {
        *CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = ServerConfig::default();
    }
}

/// One of our data folders, read on every data load (a world opening,
/// `/reload`): its files as (`<namespace>:<path>`, JSON), sorted by id.
struct DataFolder {
    id: &'static str,
}

impl ServerReloadListener for DataFolder {
    fn prepare(&self, context: &ServerReloadContext) -> Result<Box<dyn ReloadApply>, ReloadResourceError> {
        let mut files = Vec::new();
        for id in context.list() {
            let Some(bytes) = context.read(id) else {
                continue;
            };
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(json) => files.push((id.to_string(), json)),
                // As vanilla: a file that won't parse is logged and skipped.
                Err(e) => log::warn!("{}: {id} is not valid JSON, skipped: {e}", meta::NAME),
            }
        }
        let listener = self.id;
        Ok(<dyn ReloadApply>::new(move || data_read(listener, files)))
    }
}

/// The data packs' quest and advancement folders.
pub(super) fn reload_listeners(listeners: &mut dyn ServerReloadListeners) {
    for (id, folder) in [(LISTENER, validate::DATA_FOLDER), (ADVANCEMENT_LISTENER, edit::ADVANCEMENT_FOLDER)] {
        let added = Identifier::parse(id)
            .map_err(|e| format!("{e:?}"))
            .and_then(|name| Ok((name, FileToIdConverter::json(folder).map_err(|e| format!("{e:?}"))?)))
            .and_then(|(name, converter)| listeners.add_listener(name, converter, Arc::new(DataFolder { id })).map_err(|e| format!("{e:?}")));
        if let Err(e) = added {
            log::error!("{}: could not read the data packs' {folder} folder: {e}", meta::NAME);
        }
    }
}

/// A data folder was read. The quests are kept for the world about to open;
/// with a world open already (`/reload`), the pack is read again at once, as
/// `/quests reload` does.
fn data_read(listener: &str, files: Vec<(String, serde_json::Value)>) {
    if listener == ADVANCEMENT_LISTENER {
        *ADVANCEMENTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = edit::advancements(&files);
        return;
    }
    *DATA.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = files;
    if state().is_some() {
        let lines = reload();
        log::info!("{}: the data packs were read again: {}", meta::NAME, lines.join(" | "));
    }
}

/// The mod's commands in every player's command tree.
pub(super) fn commands(list: &mut Vec<GameCommand>) {
    for (usage, level, _) in commands::USAGES {
        list.push(if level == 0 { GameCommand::new(usage) } else { GameCommand::op(usage, level) });
    }
}

/// The command line a parsed invocation was typed as: its words, then its
/// arguments in order (all of ours are words or text).
fn command_line(invocation: &CommandInvocation) -> String {
    let mut words = invocation.literals.clone();
    for argument in &invocation.arguments {
        match &argument.value {
            CommandArgumentValue::String(text) => words.push(text.clone()),
            CommandArgumentValue::Integer(n) => words.push(n.to_string()),
            _ => {}
        }
    }
    words.join(" ")
}

/// `/quests ...`, run inside the player's turn: what needs every player in
/// reach waits for the tick. True when it was ours.
pub(super) fn perform(host: &mut dyn ServerHost, source: &CommandSource, invocation: &CommandInvocation) -> bool {
    if invocation.literals.first().map(String::as_str) != Some("quests") {
        return false;
    }
    let lines = match source {
        CommandSource::Player { player, permission } => {
            let line = command_line(invocation);
            match commands::parse_for(&line, permission.level()) {
                Some(Ok(command)) => run(host, *player, permission.level(), command),
                Some(Err(text)) => vec![text],
                None => return false,
            }
        }
        _ => vec!["Run /quests as a player.".to_owned()],
    };
    for line in lines {
        let _ = host.commands().feedback(Text::literal(line));
    }
    true
}

fn run(host: &mut dyn ServerHost, player: PlayerRef, permission: u8, command: Command) -> Vec<String> {
    let me = player.uuid();
    if let Command::Submit { task } = &command {
        return match pick_task(me, Verb::Submit, task) {
            Ok(task) => submit(host, me, &task),
            Err(lines) => lines,
        };
    }
    if let Command::Check { task } = &command {
        return match pick_task(me, Verb::Check, task) {
            Ok(task) => {
                if let Some(s) = state().as_mut() {
                    s.session.push(me, QuestEvent::Check { task });
                }
                Vec::new()
            }
            Err(lines) => lines,
        };
    }
    if let Command::Reload = &command {
        return reload();
    }
    if let Command::Team(team) = command {
        return party(host, me, team);
    }
    let name = player.name(host).unwrap_or_default();
    let mut guard = state();
    let Some(s) = guard.as_mut() else {
        return vec!["Horizon Quests is not running in this world.".to_owned()];
    };
    match command {
        Command::Help => {
            let mut lines = vec![meta::command_reply(&name)];
            lines.extend(commands::help(permission));
            lines
        }
        Command::Progress => s.session.summary(me),
        Command::Book => {
            s.books.push(me);
            Vec::new()
        }
        Command::Claim => {
            s.session.push(me, QuestEvent::ClaimAll);
            Vec::new()
        }
        Command::Validate => s.report.clone(),
        Command::Edit => {
            s.granted.insert(me);
            s.editors.insert(me, true);
            let _ = s.senders.entry(me).or_default().send(&editor_message(true, s.source));
            vec!["The pencil is in your quest book (bottom right): it turns edit mode on.".to_owned()]
        }
        Command::Complete { player, quest } => match target(&s.session, &player).and_then(|uuid| {
            let effects = s.session.force_complete(uuid, &quest, session::now())?;
            s.pending.extend(effects.into_iter().map(|e| (uuid, e)));
            Ok(())
        }) {
            Ok(()) => vec![format!("Completed {quest} for {player}.")],
            Err(e) => vec![e],
        },
        Command::Reset { player, quest } => match target(&s.session, &player).and_then(|uuid| s.session.reset(uuid, &quest)) {
            Ok(true) => vec![format!("Reset {quest} for {player}.")],
            Ok(false) => vec![format!("{player} had nothing to reset in {quest}.")],
            Err(e) => vec![e],
        },
        Command::ResetAll { player } => match target(&s.session, &player).and_then(|uuid| s.session.reset_all(uuid)) {
            Ok(()) => vec![format!("Reset all of {player}'s progress.")],
            Err(e) => vec![e],
        },
        Command::Check { .. } | Command::Submit { .. } | Command::Reload | Command::Team(_) => Vec::new(),
    }
}

/// `/quests team ...`: the lines for the player who ran it. What others are
/// told is sent once the lock is released; the progress that changed hands is
/// saved on the next tick.
fn party(host: &mut dyn ServerHost, me: u128, command: TeamCommand) -> Vec<String> {
    let reply = {
        let mut guard = state();
        let Some(s) = guard.as_mut() else {
            return vec!["Horizon Quests is not running in this world.".to_owned()];
        };
        let dir = s.dir.clone();
        let load = |team: &str| load_team(dir.as_deref(), team);
        match command {
            TeamCommand::Info => return s.session.team_info(me),
            TeamCommand::Create { name } => s.session.create_team(me, &name, session::now()),
            TeamCommand::Invite { player } => s.session.invite(me, &player),
            TeamCommand::Join { name } => s.session.join_team(me, &name, &load),
            TeamCommand::Leave => s.session.leave_team(me, &load),
            TeamCommand::Kick { player } => s.session.kick(me, &player, &load),
        }
    };
    match reply {
        Ok(reply) => {
            for (uuid, line) in &reply.others {
                tell(host, *uuid, line);
            }
            reply.lines
        }
        Err(line) => vec![line],
    }
}

/// The task `/quests check` or `/quests submit` acts on: what the player
/// typed, made a full id. Typed nothing: the one task open to them, else Err
/// with the list of those they can name.
fn pick_task(me: u128, verb: Verb, typed: &str) -> Result<String, Vec<String>> {
    let guard = state();
    let Some(s) = guard.as_ref() else {
        return Err(vec!["Horizon Quests is not running in this world.".to_owned()]);
    };
    if !typed.trim().is_empty() {
        return Ok(s.session.resolve(me, verb, typed));
    }
    let open = s.session.open_for(me, verb);
    let (none, header, name) = match verb {
        Verb::Check => ("No checkmark task is open to you.", "Tick one with /quests check <id>:", "check"),
        Verb::Submit => ("No task waits for items you hand in.", "Hand in with /quests submit <id>:", "submit"),
    };
    match open.as_slice() {
        [] => Err(vec![none.to_owned()]),
        [(id, _)] => Ok(id.clone()),
        _ => {
            let mut lines = vec![header.to_owned()];
            lines.extend(open.iter().map(|(id, title)| format!(" - {id} ({title})")));
            lines.push(format!("A quest's id alone is enough when it has one such task: /quests {name} <quest>."));
            Err(lines)
        }
    }
}

/// A hand-in: takes what the task still lacks from player `me`'s plain
/// stacks. Silent when it works (the book and the toasts show the progress);
/// otherwise the line saying why not.
fn submit(host: &mut dyn ServerHost, me: u128, task: &str) -> Vec<String> {
    let request = state().as_ref().map(|s| {
        let name = s.session.player(me).map(|p| p.name.clone()).unwrap_or_default();
        s.session.submit_request(me, task).map(|wanted| (wanted, name))
    });
    let (wanted, name) = match request {
        Some(Ok(found)) => found,
        Some(Err(e)) => return vec![e],
        None => return vec!["Horizon Quests is not running in this world.".to_owned()],
    };
    let Some(taken) = take_items(host, me, &name, &wanted) else {
        return vec!["You cannot hand items in right now.".to_owned()];
    };
    if taken == 0 {
        return vec![format!("You carry nothing this task takes ({}).", String::from(wanted.item))];
    }
    if let Some(s) = state().as_mut() {
        s.session.push(me, QuestEvent::Submitted { task: task.to_owned(), count: taken });
    }
    Vec::new()
}

/// Takes what `wanted` lacks from player `uuid`: their inventory is read,
/// then vanilla's `/clear` takes each item. None when their inventory cannot
/// be read right now (logging in, or dead).
fn take_items(host: &mut dyn ServerHost, uuid: u128, name: &str, wanted: &Wanted) -> Option<u64> {
    let held = stacks(host, uuid)?;
    let listed: Vec<(&str, u32, bool)> = held.iter().map(|(item, count, plain)| (item.as_str(), *count, *plain)).collect();
    let tags = |tag: &str, item: &str| in_tag(host, tag, item);
    let plan = session::hand_in_plan(wanted, &listed, &tags);
    let mut taken = 0;
    for (item, count) in plan {
        if session::clear_command(name, &item, count).is_some_and(|line| host.commands().run_command(&line)) {
            taken += u64::from(count);
        }
    }
    Some(taken)
}

/// `/quests reload`: reads the pack again. A pack whose quests.json is
/// refused leaves the one in use.
fn reload() -> Vec<String> {
    let mut guard = state();
    let Some(s) = guard.as_mut() else {
        return vec!["Horizon Quests is not running in this world.".to_owned()];
    };
    let (loaded, from, source) = load_pack(s.dir.as_deref(), s.modpack.as_deref());
    let report = commands::report(&loaded, REPORT_LIMIT);
    let refused = loaded.file.chapters.is_empty() && loaded.errors() > 0;
    let mut lines = vec![format!("Read the quests from {from}.")];
    lines.extend(report.iter().cloned());
    if refused {
        lines.push("Nothing was changed: the quests in use stay.".to_owned());
    } else {
        s.source = source;
        set_pack(s, loaded.file);
        s.report = report;
    }
    lines
}

/// The player on with this name, for an admin command.
fn target(session: &Session, name: &str) -> Result<u128, String> {
    session.find_by_name(name).ok_or_else(|| format!("{name} is not on."))
}
