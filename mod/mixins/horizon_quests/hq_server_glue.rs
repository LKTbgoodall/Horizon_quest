// The server half (single player's server thread, a LAN host, later a
// dedicated server). It keeps a `Session` (hq_session) while a world is open:
// hooks only record what happened (a kill, a craft, a level, an advancement),
// and the server tick looks at inventories and at where players stand, runs
// the engine, gives rewards, tells the players, syncs their books over mod
// messages (hq_net, channel "horizon_quests:book") and saves.
// Progress is kept per team (hq_teams): a lone player's file is their UUID's,
// a party's is its id's, and teams.json says who is in which party.
// Server hooks run on the server thread; the state is behind a Mutex. The
// tick holds it while it reads (inventories, item tags, the players' places),
// which call no hook back; it is released before any helper that changes the
// game (rewards, chat, sends, commands, taking items).
// Only hooks and helpers of Horizon's catalog are called (native-hooks.json,
// docs/native-mods.md): an engine method outside it may be private.

use super::hq_commands::{self as commands, Command, TeamCommand};
use super::hq_config::{self as config, ServerConfig};
use super::hq_logic::Book;
use super::hq_meta as meta;
use super::hq_model::{ItemMatch, Progress, RewardKind};
use super::hq_net::{self as net, ToClient, ToServer};
use super::hq_progress::{self as engine, Effect, QuestEvent, Wanted};
use super::hq_session::{self as session, Rules, Session, Verb};
use super::hq_store as store;
use super::hq_validate::{self as validate, Loaded};
use super::native_configs::{NativeConfig, NativeConfigFile, NativeConfigKind};
use super::native_inventories::NativeInventoryKind;
use super::native_reload_listeners::NativeReloadListener;
use crate::game::Game;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{ServerTickPhase, WorldPhase};
use horizon_client::text::Text;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// The quest files the data packs hold (`data/<namespace>/horizon_quests/`),
/// as the last data load handed them over: (`<namespace>:<path>`, JSON).
static DATA: Mutex<Vec<(String, serde_json::Value)>> = Mutex::new(Vec::new());

/// The id of our reload listener (API 400).
const LISTENER: &str = "horizon_quests:quests";

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
    /// Effects decided outside the tick (an admin command), carried out on
    /// the next one, where every player is in reach.
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
/// the chapters of the world's data packs (`data/<namespace>/horizon_quests/`),
/// else the example pack compiled into the bundle, unless the server's config
/// turned it off.
fn load_pack(dir: Option<&Path>) -> (Loaded, &'static str) {
    if let Some(quests) = dir.map(|d| d.join("quests"))
        && quests.join("quests.json").is_file()
    {
        return (validate::load_dir(&quests), "the world's quests folder");
    }
    let data = validate::load_data(&DATA.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
    if let Some(loaded) = data {
        return (loaded, "the data packs (data/<namespace>/horizon_quests/)");
    }
    if !server_config().example_chapter {
        let empty = validate::load_pack(None, &[], &[]);
        return (empty, "nowhere: the example chapters are turned off (example_chapter = false)");
    }
    let text = |bytes: &'static [u8]| std::str::from_utf8(bytes).unwrap_or("");
    let settings = text(horizon_client::native_data!("horizon_quests", "quests/quests.json"));
    let chapters = [
        (
            "01_getting_started.json",
            text(horizon_client::native_data!("horizon_quests", "quests/chapters/01_getting_started.json")),
        ),
        (
            "02_further_afield.json",
            text(horizon_client::native_data!("horizon_quests", "quests/chapters/02_further_afield.json")),
        ),
        ("03_iron_age.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/03_iron_age.json"))),
        ("04_diamonds.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/04_diamonds.json"))),
        ("05_the_nether.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/05_the_nether.json"))),
        ("06_the_end.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/06_the_end.json"))),
        ("07_beyond.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/07_beyond.json"))),
        ("08_farming.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/08_farming.json"))),
        ("09_monsters.json", text(horizon_client::native_data!("horizon_quests", "quests/chapters/09_monsters.json"))),
    ];
    let tables = [
        ("baker_loot.json", text(horizon_client::native_data!("horizon_quests", "quests/reward_tables/baker_loot.json"))),
        (
            "seasoned_choice.json",
            text(horizon_client::native_data!("horizon_quests", "quests/reward_tables/seasoned_choice.json")),
        ),
    ];
    (validate::load_pack(Some(settings), &chapters, &tables), "the built-in example pack")
}

/// What the server's settings ask of the session.
fn rules(config: &ServerConfig) -> Rules {
    Rules { parties: config.parties, party_size: config.party_size, detection_delay: config.detection_delay }
}

/// A new state for the open world, its pack loaded and reported in the log.
fn open(game: &Game<'_>) -> State {
    let dir = game.native_world_dir().map(|w| w.join(store::FOLDER));
    let (loaded, source) = load_pack(dir.as_deref());
    let report = commands::report(&loaded, REPORT_LIMIT);
    log::info!("{}: quests from {source}: {}", meta::NAME, report.join(" | "));
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
        pending: Vec::new(),
        senders: HashMap::new(),
        receivers: HashMap::new(),
        greeting: HashSet::new(),
        crafts: Vec::new(),
        books: Vec::new(),
    }
}

fn tell(game: &mut Game<'_>, uuid: u128, line: &str) {
    game.native_tell(uuid, Text::literal(format!("[{}] {line}", meta::NAME)));
}

/// The world opened or closed.
pub(super) fn lifecycle(game: &mut Game<'_>, phase: WorldPhase) {
    if !game.native_on(meta::ID) {
        return;
    }
    match phase {
        WorldPhase::WorldOpenReady => {
            let fresh = open(game);
            *state() = Some(fresh);
        }
        WorldPhase::WorldCloseFinal | WorldPhase::WorldCloseFailed => {
            if let Some(mut s) = state().take() {
                save(&mut s);
            }
            // The next world's data load hands its own files over.
            DATA.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
        }
        _ => {}
    }
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

pub(super) fn logged_in(game: &mut Game<'_>, player: u128, name: &str) {
    if !game.native_on(meta::ID) {
        return;
    }
    let mut guard = state();
    if guard.is_none() {
        *guard = Some(open(game));
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

pub(super) fn logged_out(game: &mut Game<'_>, player: u128) {
    if !game.native_on(meta::ID) {
        return;
    }
    if let Some(s) = state().as_mut() {
        s.senders.remove(&player);
        s.receivers.remove(&player);
        s.greeting.remove(&player);
        if let Some((team, progress)) = s.session.leave(player)
            && let Some(dir) = &s.dir
            && let Err(e) = store::save_progress(dir, &team, &progress)
        {
            log::error!("{}: could not save the progress of {team}: {e}", meta::NAME);
        }
    }
}

/// Something may have entered the player's inventory: look at it next tick.
pub(super) fn poke(game: &Game<'_>, player: u128) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
        s.session.poke(player);
    }
}

pub(super) fn mob_killed(game: &Game<'_>, player: u128, kind: &str) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
        s.session.push(player, QuestEvent::Killed { entity: session::entity_id(kind) });
    }
}

/// A player took `count` of `item` out of a crafting grid or a furnace:
/// craft tasks count it on the next tick, and their inventory is looked at.
pub(super) fn crafted(game: &Game<'_>, player: u128, item: &str, count: u32) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
        s.session.poke(player);
        s.crafts.push((player, item.to_owned(), count));
    }
}

/// A player's experience level changed to `level`.
pub(super) fn level_changed(game: &Game<'_>, player: u128, level: i32) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
        s.session.set_level(player, level);
    }
}

/// A player completed an advancement.
pub(super) fn advancement_done(game: &Game<'_>, player: u128, advancement: &str) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
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
pub(super) fn server_tick(game: &mut Game<'_>, phase: ServerTickPhase) {
    if !matches!(phase, ServerTickPhase::ServerTickEnd { .. }) || !game.native_on(meta::ID) {
        return;
    }
    let config = server_config();
    let (actions, outgoing, probes) = {
        let mut guard = state();
        let Some(s) = guard.as_mut() else {
            return;
        };
        for (uuid, item, count) in std::mem::take(&mut s.crafts) {
            let tags = |tag: &str, item: &str| in_tag(game, tag, item);
            s.session.crafted(uuid, &item, count, &tags);
        }
        // The hub's players are listed once, and only when a task waits for
        // where someone stands.
        let mut guests: Option<Vec<NativeServerPlayer>> = None;
        let mut probes = Vec::new();
        for uuid in s.session.advance() {
            look_at_inventory(game, &mut s.session, uuid);
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
            if s.session.observe(uuid) {
                let guests = guests.get_or_insert_with(|| game.native_server_players());
                if let Some((dimension, position)) = place(game, guests, uuid) {
                    s.session.at(uuid, &dimension, position);
                }
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
            tell(game, action.uuid, line);
        }
        if let Some((title, reward)) = action.reward {
            give(game, action.uuid, &action.name, title.as_deref(), reward);
        }
    }
    // Levels and advancements Horizon does not tell (the single-player
    // host's): asked of vanilla's selectors, once the lock is released.
    for (uuid, name, probe) in probes {
        let mut asks = |line: Option<String>| line.is_some_and(|line| game.native_run_command(&line));
        let level = probe.level.map(|goal| engine::probe_level(goal, &mut |l| asks(session::level_probe_command(&name, l))));
        let done: Vec<String> =
            probe.advancements.into_iter().filter(|a| asks(session::advancement_probe_command(&name, a))).collect();
        if let Some(s) = state().as_mut() {
            if let Some(level) = level {
                s.session.set_level(uuid, i32::try_from(level).unwrap_or(i32::MAX));
            }
            for advancement in done {
                s.session.advancement_done(uuid, &advancement);
            }
        }
    }
    for (uuid, frames) in outgoing {
        let mut put_back = Vec::new();
        for frame in frames {
            if let Err(_) = game.native_net_send_to_player(uuid, net::CHANNEL, &frame) {
                put_back.push(frame);
            }
        }
        if !put_back.is_empty() {
            if let Some(s) = state().as_mut()
                && let Some(sender) = s.senders.get_mut(&uuid)
            {
                sender.put_back(put_back);
            }
        }
    }
}

/// Counts what the player holds for their item tasks.
fn look_at_inventory(game: &Game<'_>, session: &mut Session, uuid: u128) {
    let wanted = session.wanted(uuid);
    if wanted.is_empty() {
        return;
    }
    let inventory = match game.native_inventory(uuid, NativeInventoryKind::Player) {
        Ok(inventory) => inventory,
        // Logging in, dead, or between two states: the next look will do.
        Err(_) => return,
    };
    let stacks: Vec<(&str, u32)> = inventory.slots.iter().flatten().map(|s| (&*s.item, s.count)).collect();
    let tags = |tag: &str, item: &str| in_tag(game, tag, item);
    for event in session::item_events(&wanted, &stacks, &tags) {
        session.push(uuid, event);
    }
}

/// Where player `uuid` stands: a guest of the hub as the hub knows them,
/// else the host, whose state is the game's own outside a player's turn.
fn place(game: &Game<'_>, guests: &[NativeServerPlayer], uuid: u128) -> Option<(String, [f64; 3])> {
    if let Some(guest) = guests.iter().find(|guest| guest.uuid == uuid) {
        return Some((guest.dimension.clone(), guest.position));
    }
    game.native_player().map(|host| (host.dimension.to_owned(), [host.x, host.y, host.z]))
}

fn in_tag(game: &Game<'_>, tag: &str, item: &str) -> bool {
    let stack = ItemStack { item: Arc::from(item), count: 1, damage: 0, extra: None };
    game.native_server_item_in_tag(&stack, tag).unwrap_or(false)
}

/// Gives a reward; `title` is what the pack calls it. Items that do not fit
/// go through vanilla's `/give`, which drops them at the player's feet.
fn give(game: &mut Game<'_>, uuid: u128, name: &str, title: Option<&str>, reward: RewardKind) {
    let given = match reward {
        RewardKind::Item { item, count, .. } => {
            let stack = ItemStack { item: Arc::from(item.as_str()), count, damage: 0, extra: None };
            match game.native_give_stack(uuid, stack) {
                None => true,
                Some(rest) => session::give_command(name, &item, rest.count).is_some_and(|line| game.native_run_command(&line)),
            }
        }
        RewardKind::Xp { points } => game.native_give_experience(uuid, points),
        RewardKind::XpLevels { levels } => game.native_give_levels(uuid, i32::try_from(levels).unwrap_or(i32::MAX)),
        RewardKind::Command { command } => {
            session::reward_command(&command, name, uuid).is_some_and(|line| game.native_run_command(&line))
        }
        RewardKind::Toast { text } => {
            // On the screen of a player whose book has talked to us, else
            // in their chat.
            let (title, text) = session::toast(title, &text);
            let toast = ToClient::Toast { title: title.clone(), text: text.clone(), icon: None };
            let sent = state()
                .as_mut()
                .and_then(|s| s.senders.get_mut(&uuid))
                .is_some_and(|sender| sender.send(&toast).is_ok());
            if !sent {
                tell(game, uuid, &session::toast_line(&title, &text));
            }
            true
        }
        // The engine draws or asks for the table's entry: what reaches the
        // glue is that entry's kind, never the table.
        RewardKind::Random { .. } | RewardKind::Choice { .. } => false,
    };
    if !given {
        log::warn!("{}: a reward for {name} could not be given", meta::NAME);
        tell(game, uuid, "A reward could not be given. Tell an operator.");
    }
}

/// A mod message from a player's client: a book request (hq_net) on channel
/// "horizon_quests:book". No player's state is swapped in here.
pub(super) fn from_client(game: &mut Game<'_>, player: u128, channel: &str, data: &[u8]) {
    if channel != net::CHANNEL || !game.native_on(meta::ID) {
        return;
    }

    let to_server = {
        let mut guard = state();
        let Some(s) = guard.as_mut() else {
            return;
        };
        let receiver = s.receivers.entry(player).or_insert_with(|| net::Receiver::new(net::SERVER_MAX_PARTS));
        match receiver.push::<ToServer>(data) {
            Ok(Some(msg)) => msg,
            Ok(None) => return,
            Err(e) => {
                log::warn!("{}: bad net frame from {player}: {e}", meta::NAME);
                return;
            }
        }
    };

    match to_server {
        ToServer::Hello { protocol, version: _ } => {
            let mut guard = state();
            let Some(s) = guard.as_mut() else {
                return;
            };
            // A book that asks again while the pack is still on its way
            // would only queue it once more.
            if !s.greeting.insert(player) {
                return;
            }
            let sender = s.senders.entry(player).or_default();
            let reply = net::greet(protocol, meta::VERSION);
            let _ = sender.send(&reply);
            if protocol == net::PROTOCOL {
                let _ = sender.send(&ToClient::Book { file: s.session.book().file().clone() });
                if let Some(progress) = s.session.progress(player) {
                    let _ = sender.send(&ToClient::Progress { player: session::player_id(player), progress: progress.clone() });
                }
                let _ = sender.send(&ToClient::Party { name: s.session.party_of(player) });
            }
        }
        ToServer::Check { .. } | ToServer::Claim { .. } | ToServer::ClaimAll => {
            // A request the engine takes as it is, on the next tick.
            if let Some(event) = net::request_event(&to_server)
                && let Some(s) = state().as_mut()
            {
                s.session.push(player, event);
            }
        }
        ToServer::Submit { task } => {
            // No player's state is swapped in here: a guest of the hub is
            // reached from outside, the host is the state in place.
            let guest = game.native_server_players().iter().any(|guest| guest.uuid == player);
            let lines = submit(game, player, &task, guest);
            if let Some(line) = lines.first() {
                let mut guard = state();
                if let Some(s) = guard.as_mut() {
                    let sender = s.senders.entry(player).or_default();
                    let _ = sender.send(&ToClient::Notice { text: line.clone() });
                }
            }
        }
    }
}

/// The mod's config file: `config/horizon_quests-server.toml`, or the
/// world's own copy in `serverconfig/`.
pub(super) fn register_configs(game: &mut Game<'_>, list: &mut Vec<NativeConfig>) {
    if game.native_on(meta::ID) {
        list.push(NativeConfig::new(meta::ID, NativeConfigKind::Server, config::DEFAULT_TEXT));
        list.push(super::hq_client_glue::config_file());
    }
}

/// A config file was read (`reloaded`: it changed on disk). Ours sets the
/// server's settings; a change made while a world is open applies at once,
/// as `/quests reload` does.
pub(super) fn config_read(game: &mut Game<'_>, file: &NativeConfigFile, reloaded: bool) {
    if file.mod_id != meta::ID || !game.native_on(meta::ID) {
        return;
    }
    if file.kind == NativeConfigKind::Client {
        super::hq_client_glue::config_read(file);
        return;
    }
    if file.kind != NativeConfigKind::Server {
        return;
    }
    let (read, problems) = config::parse(&file.text);
    for problem in &problems {
        log::warn!("{}: {}: {problem}", meta::NAME, file.path.display());
    }
    let before = std::mem::replace(&mut *CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()), read);
    if let Some(s) = state().as_mut() {
        s.session.set_rules(rules(&read));
    }
    if reloaded && before != read {
        let lines = reload();
        log::info!("{}: {} changed: {}", meta::NAME, file.path.display(), lines.join(" | "));
    }
}

/// The server stopped: the next world reads its config again.
pub(super) fn config_dropped(file: &NativeConfigFile) {
    if file.mod_id == meta::ID && file.kind == NativeConfigKind::Server {
        *CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = ServerConfig::default();
    }
}

/// The data packs' quest folder, read on every data load (a world opening,
/// `/reload`).
pub(super) fn reload_listeners(game: &mut Game<'_>, list: &mut Vec<NativeReloadListener>) {
    if game.native_on(meta::ID) {
        list.push(NativeReloadListener::json(LISTENER, validate::DATA_FOLDER));
    }
}

/// The data packs' quest files were read. Kept for the world about to open;
/// with a world open already (`/reload`), the pack is read again at once, as
/// `/quests reload` does.
pub(super) fn reload_listener_apply(game: &mut Game<'_>, listener: &NativeReloadListener, files: &[(String, serde_json::Value)]) {
    if listener.id != LISTENER || !game.native_on(meta::ID) {
        return;
    }
    *DATA.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = files.to_vec();
    if state().is_some() {
        let lines = reload();
        log::info!("{}: the data packs were read again: {}", meta::NAME, lines.join(" | "));
    }
}

/// The mod's commands in every player's command tree.
pub(super) fn commands(list: &mut Vec<NativeCommand>) {
    for (usage, level, _) in commands::USAGES {
        list.push(if level == 0 { NativeCommand::new(usage) } else { NativeCommand::op(usage, level) });
    }
}

/// `/quests ...`, run inside the player's turn: what needs every player in
/// reach waits for the tick.
pub(super) fn perform(game: &mut Game<'_>, player: &NativeServerPlayer, command: &str, ran: &mut bool) {
    if !game.native_on(meta::ID) {
        return;
    }
    let Some(parsed) = commands::parse_for(command, player.permission) else {
        return;
    };
    *ran = true;
    let lines = match parsed {
        Ok(command) => run(game, player, command),
        Err(text) => vec![text],
    };
    for line in lines {
        game.native_feedback(Text::literal(line));
    }
}

fn run(game: &mut Game<'_>, player: &NativeServerPlayer, command: Command) -> Vec<String> {
    let me = player.uuid;
    if let Command::Submit { task } = &command {
        // Inside the player's own command: their state is in place.
        return match pick_task(me, Verb::Submit, task) {
            Ok(task) => submit(game, me, &task, false),
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
        return party(game, me, team);
    }
    let mut guard = state();
    let Some(s) = guard.as_mut() else {
        return vec!["Horizon Quests is not running in this world.".to_owned()];
    };
    match command {
        Command::Help => {
            let mut lines = vec![meta::command_reply(&player.name)];
            lines.extend(commands::help(player.permission));
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
fn party(game: &mut Game<'_>, me: u128, command: TeamCommand) -> Vec<String> {
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
                tell(game, *uuid, line);
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
/// otherwise the line saying why not. `outside` says how they are reached: false when their state is in
/// place (`/quests submit` runs in their turn; the host, for a request of
/// their book), true for a guest of the hub whose book asked, outside any
/// player's turn.
fn submit(game: &mut Game<'_>, me: u128, task: &str, outside: bool) -> Vec<String> {
    let request = state().as_ref().map(|s| {
        let name = s.session.player(me).map(|p| p.name.clone()).unwrap_or_default();
        s.session.submit_request(me, task).map(|wanted| (wanted, name))
    });
    let (wanted, name) = match request {
        Some(Ok(found)) => found,
        Some(Err(e)) => return vec![e],
        None => return vec!["Horizon Quests is not running in this world.".to_owned()],
    };
    let taken = if outside {
        match take_from_guest(game, me, &name, &wanted) {
            Some(taken) => taken,
            None => return vec!["You cannot hand items in right now.".to_owned()],
        }
    } else {
        take_in_place(game, &wanted)
    };
    if taken == 0 {
        return vec![format!("You carry nothing this task takes ({}).", String::from(wanted.item))];
    }
    if let Some(s) = state().as_mut() {
        s.session.push(me, QuestEvent::Submitted { task: task.to_owned(), count: taken });
    }
    Vec::new()
}

/// Takes what `wanted` lacks from the player whose state is in place; how
/// many it took.
fn take_in_place(game: &mut Game<'_>, wanted: &Wanted) -> u64 {
    let candidates: Vec<String> = game
        .native_carried_items()
        .into_iter()
        .map(|(item, _)| item)
        .filter(|item| match &wanted.item {
            ItemMatch::Id(id) => item == id,
            ItemMatch::Tag(tag) => in_tag(game, tag, item),
        })
        .collect();
    let mut taken: u64 = 0;
    for item in candidates {
        let left = u32::try_from(wanted.missing - taken).unwrap_or(u32::MAX);
        if left == 0 {
            break;
        }
        taken += u64::from(game.native_take_items(&item, left));
    }
    taken
}

/// Takes what `wanted` lacks from a guest of the hub, outside any player's
/// turn: their inventory is read (API 132), then vanilla's `/clear` takes
/// each item (API 21). None when their inventory cannot be read right now
/// (logging in, or dead).
fn take_from_guest(game: &mut Game<'_>, uuid: u128, name: &str, wanted: &Wanted) -> Option<u64> {
    let inventory = game.native_inventory(uuid, NativeInventoryKind::Player).ok()?;
    let stacks: Vec<(&str, u32, bool)> =
        inventory.slots.iter().flatten().map(|s| (&*s.item, s.count, s.damage == 0 && s.extra.is_none())).collect();
    let tags = |tag: &str, item: &str| in_tag(game, tag, item);
    let plan = session::hand_in_plan(wanted, &stacks, &tags);
    let mut taken = 0;
    for (item, count) in plan {
        if session::clear_command(name, &item, count).is_some_and(|line| game.native_run_command(&line)) {
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
    let (loaded, source) = load_pack(s.dir.as_deref());
    let report = commands::report(&loaded, REPORT_LIMIT);
    let refused = loaded.file.chapters.is_empty() && loaded.errors() > 0;
    let mut lines = vec![format!("Read the quests from {source}.")];
    lines.extend(report.iter().cloned());
    if refused {
        lines.push("Nothing was changed: the quests in use stay.".to_owned());
    } else {
        let file = loaded.file.clone();
        s.session.set_book(Book::new(loaded.file));
        s.report = report;
        for sender in s.senders.values_mut() {
            let _ = sender.send(&ToClient::Book { file: file.clone() });
        }
    }
    lines
}

/// The player on with this name, for an admin command.
fn target(session: &Session, name: &str) -> Result<u128, String> {
    session.find_by_name(name).ok_or_else(|| format!("{name} is not on."))
}
