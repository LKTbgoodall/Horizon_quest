// The server half (single player's server thread, a LAN host, later a
// dedicated server). It keeps a `Session` (hq_session) while a world is open:
// hooks only record what happened, and the server tick looks at inventories,
// runs the engine, gives rewards, tells the players and saves. It still
// answers the test book's pings (spike 3) until the real sync replaces them.
// Server hooks run on the server thread; the state is behind a Mutex, which
// is never held while a game helper runs.

use super::hq_commands::{self as commands, Command};
use super::hq_logic::Book;
use super::hq_meta as meta;
use super::hq_model::{ItemMatch, RewardKind};
use super::hq_progress::{Effect, QuestEvent};
use super::hq_protocol as proto;
use super::hq_session::{self as session, Session};
use super::hq_store as store;
use super::hq_validate::{self as validate, Loaded};
use super::native_inventories::NativeInventoryKind;
use crate::game::Game;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{ServerTickPhase, WorldPhase};
use horizon_client::text::Text;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex, MutexGuard};

/// The pings answered since the game started.
static PONGS: AtomicU32 = AtomicU32::new(0);

/// The open world's quest state; None while no world is open.
static STATE: Mutex<Option<State>> = Mutex::new(None);

struct State {
    session: Session,
    /// `<world>/horizon-quests/`; None when the world has no folder.
    dir: Option<PathBuf>,
    /// What `/quests validate` answers for the pack in use.
    report: Vec<String>,
    /// Effects decided outside the tick (an admin command), carried out on
    /// the next one, where every player is in reach.
    pending: Vec<(u128, Effect)>,
}

/// How many problems a report lists in chat.
const REPORT_LIMIT: usize = 8;

fn state() -> MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The pack: `<world>/horizon-quests/quests/` when the world has one, else
/// the example pack compiled into the bundle.
fn load_pack(dir: Option<&Path>) -> (Loaded, &'static str) {
    if let Some(quests) = dir.map(|d| d.join("quests"))
        && quests.join("quests.json").is_file()
    {
        return (validate::load_dir(&quests), "the world's quests folder");
    }
    let settings = horizon_client::native_data!("horizon_quests", "quests/quests.json");
    let chapter = horizon_client::native_data!("horizon_quests", "quests/chapters/01_getting_started.json");
    let text = |bytes: &'static [u8]| std::str::from_utf8(bytes).unwrap_or("");
    let loaded = validate::load(Some(text(settings)), &[("chapters/01_getting_started.json", text(chapter))]);
    (loaded, "the built-in example pack")
}

/// A new state for the open world, its pack loaded and reported in the log.
fn open(game: &Game<'_>) -> State {
    let dir = game.native_world_dir().map(|w| w.join(store::FOLDER));
    let (loaded, source) = load_pack(dir.as_deref());
    let report = commands::report(&loaded, REPORT_LIMIT);
    log::info!("{}: quests from {source}: {}", meta::NAME, report.join(" | "));
    State { session: Session::new(Book::new(loaded.file)), dir, report, pending: Vec::new() }
}

fn tell(game: &mut Game<'_>, uuid: u128, line: &str) {
    game.native_tell(uuid, Text::literal(format!("[{}] {line}", meta::NAME)));
}

/// The world opened or closed.
pub fn lifecycle(game: &mut Game<'_>, phase: WorldPhase) {
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
        }
        _ => {}
    }
}

/// Writes every changed progress.
fn save(s: &mut State) {
    let Some(dir) = s.dir.clone() else {
        return;
    };
    for (uuid, progress) in s.session.take_dirty() {
        if let Err(e) = store::save_progress(&dir, &store::uuid_string(uuid), &progress) {
            log::error!("{}: could not save the progress of {}: {e}", meta::NAME, store::uuid_string(uuid));
        }
    }
}

pub fn logged_in(game: &mut Game<'_>, player: u128, name: &str) {
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
    let progress = match &s.dir {
        Some(dir) => store::load_progress(dir, &store::uuid_string(player)),
        None => Ok(Default::default()),
    };
    if let Err(e) = &progress {
        log::error!("{}: {name}'s progress is not readable, it will not be saved: {e}", meta::NAME);
    }
    s.session.join(player, name, progress);
}

pub fn logged_out(game: &mut Game<'_>, player: u128) {
    if !game.native_on(meta::ID) {
        return;
    }
    if let Some(s) = state().as_mut()
        && let Some(progress) = s.session.leave(player)
        && let Some(dir) = &s.dir
        && let Err(e) = store::save_progress(dir, &store::uuid_string(player), &progress)
    {
        log::error!("{}: could not save the progress of {}: {e}", meta::NAME, store::uuid_string(player));
    }
}

/// Something may have entered the player's inventory: look at it next tick.
pub fn poke(game: &Game<'_>, player: u128) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
        s.session.poke(player);
    }
}

pub fn mob_killed(game: &Game<'_>, player: u128, kind: &str) {
    if game.native_on(meta::ID)
        && let Some(s) = state().as_mut()
    {
        s.session.push(player, QuestEvent::Killed { entity: session::entity_id(kind) });
    }
}

/// What a tick decided, carried out once the lock is released.
struct Action {
    uuid: u128,
    name: String,
    line: Option<String>,
    reward: Option<RewardKind>,
}

/// The end of every server tick: inventories, engine, effects, saving.
pub fn server_tick(game: &mut Game<'_>, phase: ServerTickPhase) {
    if !matches!(phase, ServerTickPhase::ServerTickEnd { .. }) || !game.native_on(meta::ID) {
        return;
    }
    let actions = {
        let mut guard = state();
        let Some(s) = guard.as_mut() else {
            return;
        };
        for uuid in s.session.advance() {
            look_at_inventory(game, &mut s.session, uuid);
        }
        let mut effects = std::mem::take(&mut s.pending);
        effects.extend(s.session.process(session::now()));
        let book = s.session.book();
        let actions: Vec<Action> = effects
            .into_iter()
            .map(|(uuid, effect)| Action {
                uuid,
                name: s.session.player(uuid).map(|p| p.name.clone()).unwrap_or_default(),
                line: commands::announce(book, &effect),
                reward: match effect {
                    Effect::Reward { kind, .. } => Some(kind),
                    _ => None,
                },
            })
            .collect();
        save(s);
        actions
    };
    for action in actions {
        if let Some(line) = &action.line {
            tell(game, action.uuid, line);
        }
        if let Some(reward) = action.reward {
            give(game, action.uuid, &action.name, reward);
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

fn in_tag(game: &Game<'_>, tag: &str, item: &str) -> bool {
    let stack = ItemStack { item: Arc::from(item), count: 1, damage: 0, extra: None };
    game.native_server_item_in_tag(&stack, tag).unwrap_or(false)
}

/// Gives a reward. Items that do not fit go through vanilla's `/give`,
/// which drops them at the player's feet.
fn give(game: &mut Game<'_>, uuid: u128, name: &str, reward: RewardKind) {
    let given = match reward {
        RewardKind::Item { item, count } => {
            let stack = ItemStack { item: Arc::from(item.as_str()), count, damage: 0, extra: None };
            match game.native_give_stack(uuid, stack) {
                None => true,
                Some(rest) => session::give_command(name, &item, rest.count).is_some_and(|line| game.native_run_command(&line)),
            }
        }
        RewardKind::Xp { points } => game.native_give_experience(uuid, points),
        RewardKind::XpLevels { levels } => game.native_give_levels(uuid, i32::try_from(levels).unwrap_or(i32::MAX)),
    };
    if !given {
        log::warn!("{}: a reward for {name} could not be given", meta::NAME);
        tell(game, uuid, "A reward could not be given. Tell an operator.");
    }
}

/// A mod message from a player's client: answer a ping with its pong.
pub fn from_client(game: &mut Game<'_>, player: u128, channel: &str, data: &[u8]) {
    if channel != proto::CHANNEL || !game.native_on(meta::ID) {
        return;
    }
    let Some(reply) = proto::decode(data).and_then(proto::answer) else {
        return;
    };
    match game.native_net_send_to_player(player, proto::CHANNEL, &proto::encode(reply)) {
        Ok(()) => {
            PONGS.fetch_add(1, Relaxed);
        }
        Err(e) => log::warn!("{}: could not answer a ping: {e}", meta::NAME),
    }
}

/// The mod's commands in every player's command tree.
pub fn commands(list: &mut Vec<NativeCommand>) {
    for (usage, level, _) in commands::USAGES {
        list.push(if level == 0 { NativeCommand::new(usage) } else { NativeCommand::op(usage, level) });
    }
}

/// `/quests ...`, run inside the player's turn: what needs every player in
/// reach waits for the tick.
pub fn perform(game: &mut Game<'_>, player: &NativeServerPlayer, command: &str, ran: &mut bool) {
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
        return submit(game, me, task);
    }
    if let Command::Reload = &command {
        return reload();
    }
    let mut guard = state();
    let Some(s) = guard.as_mut() else {
        return vec!["Horizon Quests is not running in this world.".to_owned()];
    };
    match command {
        Command::Help => {
            let mut lines = vec![meta::command_reply(&player.name, PONGS.load(Relaxed))];
            lines.extend(commands::help(player.permission));
            lines
        }
        Command::Progress => s.session.summary(me),
        Command::Claim => {
            s.session.push(me, QuestEvent::ClaimAll);
            Vec::new()
        }
        Command::Check { task } => {
            s.session.push(me, QuestEvent::Check { task });
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
        Command::Submit { .. } | Command::Reload => Vec::new(),
    }
}

/// `/quests submit`: takes what the task still lacks from the player's
/// plain stacks (the player's state is in place: this is their turn).
fn submit(game: &mut Game<'_>, me: u128, task: &str) -> Vec<String> {
    let wanted = match state().as_ref().map(|s| s.session.submit_request(me, task)) {
        Some(Ok(wanted)) => wanted,
        Some(Err(e)) => return vec![e],
        None => return vec!["Horizon Quests is not running in this world.".to_owned()],
    };
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
    if taken == 0 {
        return vec![format!("You carry nothing this task takes ({}).", String::from(wanted.item))];
    }
    if let Some(s) = state().as_mut() {
        s.session.push(me, QuestEvent::Submitted { task: task.to_owned(), count: taken });
    }
    vec![format!("Handed in {taken}.")]
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
        s.session.set_book(Book::new(loaded.file));
        s.report = report;
    }
    lines
}

/// The player on with this name, for an admin command.
fn target(session: &Session, name: &str) -> Result<u128, String> {
    session.find_by_name(name).ok_or_else(|| format!("{name} is not on."))
}
