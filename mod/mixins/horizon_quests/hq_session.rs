// The server's quest state while a world is open: the pack, the progress of
// every player on, the events waiting for the next tick, and when to look at
// each player's inventory. The glue feeds it hooks and inventories, calls
// `process` on the server tick and carries out the effects it returns. Pure:
// no game types.
//
// Item tasks are found two ways at once (ADR-010): every player's inventory
// is looked at every `detection_delay` ticks, so nothing is missed however the
// items came (a chest, a trade, /give, items held before the quest opened);
// and a pickup, a craft or a furnace take `poke`s the player, so the next
// tick looks at once and the book reacts without waiting.
use super::hq_logic::{Book, QuestState};
use super::hq_model::{ItemMatch, Progress, TaskKind, resource_id};
use super::hq_progress::{self as engine, Actor, Effect, QuestEvent, Wanted};
use std::collections::{HashMap, HashSet};

/// A player on the server.
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Player {
    pub name: String,
    pub progress: Progress,
    /// Changed since it was last saved.
    pub dirty: bool,
    /// Changed since it was last sent to their book. Apart from `dirty`: a
    /// progress that is never saved is still sent once per change.
    pub unsynced: bool,
    /// False when their file could not be read: it is never overwritten.
    pub saveable: bool,
    /// Their experience level, once a change of it was heard.
    pub level: Option<u32>,
}

impl Player {
    /// The engine said whether the progress changed.
    fn changed(&mut self, changed: bool) {
        self.dirty |= changed;
        self.unsynced |= changed;
    }
}

#[derive(Debug)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Session {
    book: Book,
    players: HashMap<u128, Player>,
    queue: Vec<(u128, QuestEvent)>,
    tick: u64,
    /// The tick of each player's next inventory look.
    next_look: HashMap<u128, u64>,
    /// Players to look at on the next tick whatever their turn.
    poked: HashSet<u128>,
    /// The state of the random bits handed to the engine (random rewards).
    roll: u64,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Session {
    pub fn new(book: Book) -> Self {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        Self {
            book,
            players: HashMap::new(),
            queue: Vec::new(),
            tick: 0,
            next_look: HashMap::new(),
            poked: HashSet::new(),
            roll: seed,
        }
    }

    pub fn book(&self) -> &Book {
        &self.book
    }

    /// A new pack (`/quests reload`): every player's progress is checked
    /// against it on the next tick.
    pub fn set_book(&mut self, book: Book) {
        self.book = book;
        let uuids: Vec<u128> = self.players.keys().copied().collect();
        for uuid in uuids {
            self.push(uuid, QuestEvent::Refresh);
            self.poke(uuid);
        }
    }

    /// A player joined, with their progress as the store read it. An Err
    /// keeps them playing on an empty progress that is never saved, so the
    /// unreadable file stays for someone to repair.
    pub fn join(&mut self, uuid: u128, name: &str, progress: Result<Progress, String>) {
        let (progress, saveable) = match progress {
            Ok(progress) => (progress, true),
            Err(_) => (Progress::default(), false),
        };
        let player = Player { name: name.to_owned(), progress, dirty: false, unsynced: false, saveable, level: None };
        self.players.insert(uuid, player);
        self.push(uuid, QuestEvent::Refresh);
        self.poke(uuid);
    }

    /// A player left: what to save for them, if anything.
    pub fn leave(&mut self, uuid: u128) -> Option<Progress> {
        self.next_look.remove(&uuid);
        self.poked.remove(&uuid);
        self.queue.retain(|(who, _)| *who != uuid);
        let player = self.players.remove(&uuid)?;
        (player.dirty && player.saveable).then_some(player.progress)
    }

    pub fn player(&self, uuid: u128) -> Option<&Player> {
        self.players.get(&uuid)
    }

    /// The player on with this name, ignoring case.
    pub fn find_by_name(&self, name: &str) -> Option<u128> {
        self.players.iter().find(|(_, p)| p.name.eq_ignore_ascii_case(name)).map(|(uuid, _)| *uuid)
    }

    /// Something a player did, for the next tick.
    pub fn push(&mut self, uuid: u128, event: QuestEvent) {
        if self.players.contains_key(&uuid) {
            self.queue.push((uuid, event));
        }
    }

    /// Look at this player's inventory on the next tick.
    pub fn poke(&mut self, uuid: u128) {
        if self.players.contains_key(&uuid) {
            self.poked.insert(uuid);
        }
    }

    /// The player took `count` of `item` out of a crafting grid or a
    /// furnace: reports each craft task it counts for.
    pub fn crafted(&mut self, uuid: u128, item: &str, count: u32, in_tag: &dyn Fn(&str, &str) -> bool) {
        let Some(player) = self.players.get(&uuid) else {
            return;
        };
        let Some(item) = resource_id(item) else {
            return;
        };
        for wanted in engine::wanted_crafts(&self.book, &player.progress) {
            let counts = match &wanted.item {
                ItemMatch::Id(id) => *id == item,
                ItemMatch::Tag(tag) => in_tag(tag, &item),
            };
            if counts {
                self.push(uuid, QuestEvent::Crafted { task: wanted.task, count: u64::from(count) });
            }
        }
    }

    /// The player's experience level changed to `level`.
    pub fn set_level(&mut self, uuid: u128, level: i32) {
        let level = u32::try_from(level).unwrap_or(0);
        if let Some(player) = self.players.get_mut(&uuid) {
            player.level = Some(level);
            self.push(uuid, QuestEvent::Level { level });
        }
    }

    /// The player completed an advancement.
    pub fn advancement_done(&mut self, uuid: u128, advancement: &str) {
        self.push(uuid, QuestEvent::Advancement { advancement: advancement.to_owned() });
    }

    /// A look at a player, beyond their inventory: reports their level
    /// again when a task waits for it (the task may have opened since the
    /// level last changed) and wakes quests waiting to start over. Whether a
    /// task waits for where they stand: report it with `at`.
    pub fn observe(&mut self, uuid: u128) -> bool {
        let Some(player) = self.players.get(&uuid) else {
            return false;
        };
        let watch = engine::watch(&self.book, &player.progress);
        let level = player.level;
        if watch.repeat {
            self.push(uuid, QuestEvent::Refresh);
        }
        if watch.level
            && let Some(level) = level
        {
            self.push(uuid, QuestEvent::Level { level });
        }
        watch.place
    }

    /// The player stands at `position` in `dimension`.
    pub fn at(&mut self, uuid: u128, dimension: &str, position: [f64; 3]) {
        let block = |v: f64| v.floor().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        let position = [block(position[0]), block(position[1]), block(position[2])];
        self.push(uuid, QuestEvent::At { dimension: entity_id(dimension), position });
    }

    /// One server tick went by: the players to look at now (their
    /// inventory, where they stand).
    pub fn advance(&mut self) -> Vec<u128> {
        self.tick += 1;
        let delay = u64::from(self.book.settings().detection_delay.max(1));
        let mut due = Vec::new();
        for &uuid in self.players.keys() {
            let next = self.next_look.entry(uuid).or_insert(self.tick);
            if self.poked.contains(&uuid) || *next <= self.tick {
                *next = self.tick + delay;
                due.push(uuid);
            }
        }
        self.poked.clear();
        due.sort_unstable();
        due
    }

    /// The item tasks this player can progress by holding items.
    pub fn wanted(&self, uuid: u128) -> Vec<Wanted> {
        let Some(player) = self.players.get(&uuid) else {
            return Vec::new();
        };
        engine::wanted_items(&self.book, &player.progress).into_iter().filter(|w| !w.consume).collect()
    }

    /// What a hand-in for `task` should take from this player.
    pub fn submit_request(&self, uuid: u128, task: &str) -> Result<Wanted, String> {
        let player = self.players.get(&uuid).ok_or("You are not in the quest book yet.")?;
        engine::submit_request(&self.book, &player.progress, task).map_err(|r| r.text().to_owned())
    }

    /// Runs every waiting event through the engine, at `now` (Unix seconds):
    /// what to carry out, for whom.
    pub fn process(&mut self, now: u64) -> Vec<(u128, Effect)> {
        let mut effects = Vec::new();
        for (uuid, event) in std::mem::take(&mut self.queue) {
            let Some(player) = self.players.get_mut(&uuid) else {
                continue;
            };
            let roll = engine::next_roll(&mut self.roll);
            let id = player_id(uuid);
            let who = Actor::new(&id, now).with_roll(roll);
            let outcome = engine::apply(&self.book, &mut player.progress, &who, &event);
            player.changed(outcome.changed);
            effects.extend(outcome.effects.into_iter().map(|e| (uuid, e)));
        }
        effects
    }

    /// The progress to write now: every changed one that may be saved.
    pub fn take_dirty(&mut self) -> Vec<(u128, Progress)> {
        let mut out = Vec::new();
        for (&uuid, player) in &mut self.players {
            if player.dirty && player.saveable {
                player.dirty = false;
                out.push((uuid, player.progress.clone()));
            }
        }
        out.sort_unstable_by_key(|(uuid, _)| *uuid);
        out
    }

    /// The progress to send to the books now: every one changed since it
    /// was last sent, with the id its claims are recorded under.
    pub fn take_unsynced(&mut self) -> Vec<(u128, String, Progress)> {
        let mut out = Vec::new();
        for (&uuid, player) in &mut self.players {
            if player.unsynced {
                player.unsynced = false;
                out.push((uuid, player_id(uuid), player.progress.clone()));
            }
        }
        out.sort_unstable_by_key(|(uuid, _, _)| *uuid);
        out
    }

    /// `/quests complete`: what it did, or why not.
    pub fn force_complete(&mut self, uuid: u128, quest: &str, now: u64) -> Result<Vec<Effect>, String> {
        let roll = engine::next_roll(&mut self.roll);
        let player = self.players.get_mut(&uuid).ok_or("That player is not on.")?;
        let id = player_id(uuid);
        let who = Actor::new(&id, now).with_roll(roll);
        let outcome = engine::force_complete(&self.book, &mut player.progress, &who, quest)
            .ok_or_else(|| format!("There is no quest `{quest}`."))?;
        player.changed(outcome.changed);
        Ok(outcome.effects)
    }

    /// `/quests reset`: whether there was anything to reset.
    pub fn reset(&mut self, uuid: u128, quest: &str) -> Result<bool, String> {
        let player = self.players.get_mut(&uuid).ok_or("That player is not on.")?;
        let changed = engine::reset_quest(&self.book, &mut player.progress, quest)
            .ok_or_else(|| format!("There is no quest `{quest}`."))?;
        player.changed(changed);
        self.poke(uuid);
        Ok(changed)
    }

    /// `/quests reset_all`.
    pub fn reset_all(&mut self, uuid: u128) -> Result<(), String> {
        let player = self.players.get_mut(&uuid).ok_or("That player is not on.")?;
        player.progress = Progress::default();
        player.changed(true);
        self.push(uuid, QuestEvent::Refresh);
        self.poke(uuid);
        Ok(())
    }

    /// `/quests progress`: each chapter's tally, then the quests this player
    /// can work on now or has rewards waiting in.
    pub fn summary(&self, uuid: u128) -> Vec<String> {
        let Some(player) = self.players.get(&uuid) else {
            return vec!["You are not in the quest book yet.".to_owned()];
        };
        let book = &self.book;
        let statuses = book.statuses(&player.progress, &player_id(uuid));
        let mut lines = Vec::new();
        for c in 0..book.file().chapters.len() {
            let chapter = book.chapter(c);
            let (done, total) = book.chapter_tally(c, &player.progress);
            lines.push(format!("{}: {done}/{total} quests", label(&chapter.title, &chapter.id)));
            for n in book.chapter_nodes(c) {
                let status = statuses[n];
                let state = match status.state {
                    QuestState::Available => "open",
                    QuestState::InProgress => "in progress",
                    QuestState::RewardsWaiting => "rewards waiting (/quests claim)",
                    QuestState::Locked | QuestState::Done => continue,
                };
                if status.visible {
                    let quest = book.quest(n);
                    lines.push(format!(" - {}: {state}{}", label(&quest.title, &book.nodes()[n].id), tasks(book, &player.progress, n)));
                }
            }
        }
        lines
    }
}

/// How far the quest's tasks are: ` (8/16 minecraft:oak_log, 0/1 checkmark)`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn tasks(book: &Book, progress: &Progress, n: usize) -> String {
    let parts: Vec<String> = book
        .quest(n)
        .tasks
        .iter()
        .enumerate()
        .map(|(t, task)| {
            let what = match &task.kind {
                TaskKind::Item { item, .. } => String::from(item.clone()),
                TaskKind::Kill { entity, .. } => format!("kill {entity}"),
                TaskKind::Checkmark => format!("check: /quests check {}", book.task_id(n, t)),
                TaskKind::Craft { item, .. } => format!("craft {}", String::from(item.clone())),
                TaskKind::XpLevels { .. } => "experience levels".to_owned(),
                TaskKind::Dimension { dimension } => format!("go to {dimension}"),
                TaskKind::Location { .. } => "reach a place (see the quest book)".to_owned(),
                TaskKind::Advancement { advancement } => format!("advancement {advancement}"),
            };
            format!("{}/{} {what}", progress.task(&book.task_id(n, t)).min(task.kind.goal()), task.kind.goal())
        })
        .collect();
    if parts.is_empty() { String::new() } else { format!(" ({})", parts.join(", ")) }
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn label<'a>(title: &'a str, id: &'a str) -> &'a str {
    if title.is_empty() { id } else { title }
}

/// A player as the engine names them: claims are kept per player under
/// this id, and their book looks its claims up under the same one.
pub fn player_id(uuid: u128) -> String {
    super::hq_store::uuid_string(uuid)
}

/// How many of the stacks (`(item id, count)`) `item` matches. `in_tag(tag,
/// item)` answers for tags, from the world's item tags.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn count_item(stacks: &[(&str, u32)], item: &ItemMatch, in_tag: &dyn Fn(&str, &str) -> bool) -> u64 {
    stacks
        .iter()
        .filter(|(id, _)| match item {
            ItemMatch::Id(want) => id == want,
            ItemMatch::Tag(tag) => in_tag(tag, id),
        })
        .map(|(_, count)| u64::from(*count))
        .sum()
}

/// What an inventory look reports for the tasks `wanted`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn item_events(wanted: &[Wanted], stacks: &[(&str, u32)], in_tag: &dyn Fn(&str, &str) -> bool) -> Vec<QuestEvent> {
    wanted
        .iter()
        .map(|w| QuestEvent::ItemCount { task: w.task.clone(), count: count_item(stacks, &w.item, in_tag) })
        .filter(|e| matches!(e, QuestEvent::ItemCount { count, .. } if *count > 0))
        .collect()
}

/// An entity type as kill tasks name it (`zombie` → `minecraft:zombie`).
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn entity_id(kind: &str) -> String {
    resource_id(kind).unwrap_or_else(|| kind.to_owned())
}

/// The console line that gives what a full inventory left over: vanilla's
/// `/give` drops it at the player's feet. None for a name or an item that
/// could not be written safely on a command line.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn give_command(name: &str, item: &str, count: u32) -> Option<String> {
    let item = resource_id(item)?;
    (safe_name(name) && count > 0).then(|| format!("give {name} {item} {count}"))
}

/// Whether `name` is a player name that can be written on a command line.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn safe_name(name: &str) -> bool {
    (1..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The console line of a command reward for this player: `{player}` is
/// their name and `{uuid}` their UUID. None for a name that could not be
/// written safely on a command line, or a command that is not one line.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn reward_command(command: &str, name: &str, uuid: u128) -> Option<String> {
    let line = command.trim().trim_start_matches('/');
    if !safe_name(name) || line.is_empty() || line.contains(['\n', '\r']) {
        return None;
    }
    Some(line.replace("{player}", name).replace("{uuid}", &player_id(uuid)))
}

/// What a hand-in for `wanted` takes from an inventory read as stacks
/// `(item id, count, plain)`: each item and how many of it, in the order the
/// items are first met, never more than the task lacks. An item the player
/// also holds in a stack that is not plain (damaged, named, enchanted) is
/// left alone: the take goes by item id and could not spare that stack.
pub fn hand_in_plan(wanted: &Wanted, stacks: &[(&str, u32, bool)], in_tag: &dyn Fn(&str, &str) -> bool) -> Vec<(String, u32)> {
    let mut held: Vec<(&str, u64, bool)> = Vec::new();
    for &(item, count, plain) in stacks {
        match held.iter_mut().find(|(id, _, _)| *id == item) {
            Some((_, total, all_plain)) => {
                *total += u64::from(count);
                *all_plain &= plain;
            }
            None => held.push((item, u64::from(count), plain)),
        }
    }
    let mut left = wanted.missing;
    let mut plan = Vec::new();
    for (item, total, all_plain) in held {
        let counts = match &wanted.item {
            ItemMatch::Id(id) => id == item,
            ItemMatch::Tag(tag) => in_tag(tag, item),
        };
        let take = u32::try_from(total.min(left)).unwrap_or(u32::MAX);
        if counts && all_plain && take > 0 {
            left -= u64::from(take);
            plan.push((item.to_owned(), take));
        }
    }
    plan
}

/// The console line that takes `count` of `item` from a player: vanilla's
/// `/clear`. None for a name or an item that could not be written safely on
/// a command line.
pub fn clear_command(name: &str, item: &str, count: u32) -> Option<String> {
    let item = resource_id(item)?;
    (safe_name(name) && count > 0).then(|| format!("clear {name} {item} {count}"))
}

/// What a toast reward shows: its title (the reward's `title`, else a plain
/// heading) and its text.
pub fn toast(title: Option<&str>, text: &str) -> (String, String) {
    let title = title.map(str::trim).filter(|title| !title.is_empty()).unwrap_or("Quest reward");
    (title.to_owned(), text.trim().to_owned())
}

/// The chat line that stands for a toast, for a player whose client has no
/// book to show it.
pub fn toast_line(title: &str, text: &str) -> String {
    if text.is_empty() { title.to_owned() } else { format!("{title}: {text}") }
}

/// Unix seconds now.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::hq_validate::load;

    const PACK: &str = r##"{ "id": "c", "title": "Start", "quests": [
        { "id": "logs", "title": "Logs", "tasks": [ { "id": "t", "type": "item", "item": "#minecraft:logs", "count": 4 } ],
          "rewards": [ { "id": "r", "type": "xp", "points": 5 } ] },
        { "id": "bread", "dependencies": ["logs"], "tasks": [ { "id": "t", "type": "item", "item": "bread" } ] },
        { "id": "zombie", "tasks": [ { "id": "t", "type": "kill", "entity": "zombie", "count": 2 } ] }
    ] }"##;

    fn session() -> Session {
        let loaded = load(Some(r#"{ "detection_delay": 10 }"#), &[("chapters/c.json", PACK)]);
        assert_eq!(loaded.errors(), 0, "{:?}", loaded.problems);
        Session::new(Book::new(loaded.file))
    }

    fn logs(tag: &str, item: &str) -> bool {
        tag == "minecraft:logs" && item.ends_with("_log")
    }

    #[test]
    fn counts_ids_and_tags() {
        let stacks = [("minecraft:oak_log", 3), ("minecraft:birch_log", 2), ("minecraft:bread", 1)];
        assert_eq!(count_item(&stacks, &ItemMatch::Tag("minecraft:logs".into()), &logs), 5);
        assert_eq!(count_item(&stacks, &ItemMatch::Id("minecraft:bread".into()), &logs), 1);
        assert_eq!(count_item(&stacks, &ItemMatch::Id("minecraft:stone".into()), &logs), 0);
    }

    #[test]
    fn looks_on_join_then_every_delay_and_when_poked() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        assert_eq!(s.advance(), [7]); // tick 1: joined, poked
        for _ in 2..=10 {
            assert!(s.advance().is_empty());
        }
        assert_eq!(s.advance(), [7]); // tick 11
        s.poke(7);
        assert_eq!(s.advance(), [7]); // tick 12: poked
        assert!(s.advance().is_empty());
    }

    #[test]
    fn an_inventory_look_completes_an_item_quest_and_saves() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.process(1);
        let wanted = s.wanted(7);
        assert_eq!(wanted.len(), 1, "only the open quest is wanted: {wanted:?}");
        let stacks = [("minecraft:oak_log", 4)];
        for event in item_events(&wanted, &stacks, &logs) {
            s.push(7, event);
        }
        let effects = s.process(2);
        assert!(effects.contains(&(7, Effect::QuestCompleted { quest: "c/logs".into() })), "{effects:?}");
        let saved = s.take_dirty();
        assert_eq!(saved.len(), 1);
        assert!(saved[0].1.is_completed("c/logs"));
        assert!(s.take_dirty().is_empty());
        // The next quest opened.
        assert_eq!(s.wanted(7)[0].task, "c/bread/t");
    }

    #[test]
    fn kills_count_by_entity_type() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.push(7, QuestEvent::Killed { entity: entity_id("zombie") });
        s.push(7, QuestEvent::Killed { entity: entity_id("minecraft:zombie") });
        let effects = s.process(1);
        assert!(effects.contains(&(7, Effect::QuestCompleted { quest: "c/zombie".into() })), "{effects:?}");
    }

    #[test]
    fn changed_progress_is_sent_once_under_the_claim_id() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.push(7, QuestEvent::Killed { entity: "minecraft:zombie".into() });
        assert!(s.take_unsynced().is_empty());
        s.process(1);
        let unsynced = s.take_unsynced();
        assert_eq!(unsynced.len(), 1);
        assert_eq!(unsynced[0].0, 7);
        // The id claims are recorded under, not the display name.
        assert_eq!(unsynced[0].1, player_id(7));
        assert!(s.take_unsynced().is_empty());
        // Saving is tracked apart: the change still waits to be written.
        assert_eq!(s.take_dirty().len(), 1);
    }

    #[test]
    fn a_claim_shows_as_claimed_under_the_id_sent_to_the_book() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.process(1);
        let stacks = [("minecraft:oak_log", 4)];
        for event in item_events(&s.wanted(7), &stacks, &logs) {
            s.push(7, event);
        }
        s.process(2);
        s.push(7, QuestEvent::ClaimAll);
        s.process(3);
        let (_, id, progress) = s.take_unsynced().pop().unwrap();
        assert!(progress.has_claimed("c/logs/r", &id), "{progress:?}");
        assert!(!progress.has_claimed("c/logs/r", "Steve"));
    }

    #[test]
    fn turning_the_pack_off_and_on_keeps_the_progress() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.push(7, QuestEvent::Killed { entity: "minecraft:zombie".into() });
        s.push(7, QuestEvent::Killed { entity: "minecraft:zombie".into() });
        s.process(1);
        assert!(s.player(7).unwrap().progress.is_completed("c/zombie"));
        // example_chapter = false: an empty pack, refreshed for everyone.
        let full = s.book().clone();
        s.set_book(Book::new(load(None, &[]).file));
        s.process(2);
        assert!(s.player(7).unwrap().progress.is_completed("c/zombie"));
        s.set_book(full);
        s.process(3);
        assert!(s.player(7).unwrap().progress.is_completed("c/zombie"));
    }

    #[test]
    fn a_progress_never_saved_is_still_sent_only_once_per_change() {
        let mut s = session();
        s.join(7, "Steve", Err("bad file".into()));
        s.push(7, QuestEvent::Killed { entity: "minecraft:zombie".into() });
        s.process(1);
        assert!(s.take_dirty().is_empty());
        assert_eq!(s.take_unsynced().len(), 1);
        assert!(s.take_unsynced().is_empty());
    }

    #[test]
    fn an_unreadable_file_is_never_saved() {
        let mut s = session();
        s.join(7, "Steve", Err("bad file".into()));
        s.push(7, QuestEvent::Killed { entity: "minecraft:zombie".into() });
        s.process(1);
        assert!(s.take_dirty().is_empty());
        assert_eq!(s.leave(7), None);
    }

    #[test]
    fn leaving_hands_back_unsaved_progress_and_forgets_the_player() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.push(7, QuestEvent::Killed { entity: "minecraft:zombie".into() });
        s.process(1);
        assert!(s.leave(7).is_some());
        s.push(7, QuestEvent::Refresh);
        assert!(s.process(2).is_empty());
        assert!(s.advance().is_empty());
    }

    #[test]
    fn admin_commands_find_players_by_name() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        assert_eq!(s.find_by_name("steve"), Some(7));
        assert!(s.force_complete(7, "c/zombie", 1).unwrap().contains(&Effect::QuestCompleted { quest: "c/zombie".into() }));
        assert_eq!(s.reset(7, "c/zombie"), Ok(true));
        assert!(s.force_complete(7, "c/nope", 1).is_err());
        assert!(s.reset_all(7).is_ok());
        assert!(s.force_complete(8, "c/zombie", 1).is_err());
    }

    #[test]
    fn the_summary_lists_open_quests_with_their_tasks() {
        let mut s = session();
        s.join(7, "Steve", Ok(Progress::default()));
        s.process(1);
        let lines = s.summary(7);
        assert_eq!(lines[0], "Start: 0/3 quests");
        assert!(lines.iter().any(|l| l == " - Logs: open (0/4 #minecraft:logs)"), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("bread")), "a locked quest is not listed: {lines:?}");
    }

    /// Crafts by id and by tag, a level, a level behind a dependency, and a
    /// place behind another.
    const WORK: &str = r##"{ "id": "w", "title": "Work", "quests": [
        { "id": "bake", "title": "Bake", "x": 0,
          "tasks": [ { "id": "t", "type": "craft", "item": "bread", "count": 2 } ] },
        { "id": "burn", "title": "Burn", "x": 1,
          "tasks": [ { "id": "t", "type": "craft", "item": "#minecraft:logs", "count": 2 } ] },
        { "id": "grow", "title": "Grow", "x": 2,
          "tasks": [ { "id": "t", "type": "xp_levels", "levels": 3 } ] },
        { "id": "walk", "title": "Walk", "x": 3, "dependencies": ["grow"],
          "tasks": [ { "id": "t", "type": "location", "dimension": "overworld", "from": [0, 60, 0], "to": [4, 70, 4] } ] },
        { "id": "later", "title": "Later", "x": 4, "dependencies": ["burn"],
          "tasks": [ { "id": "t", "type": "xp_levels", "levels": 1 } ] }
    ] }"##;

    fn work() -> Session {
        let loaded = load(None, &[("w.json", WORK)]);
        assert_eq!(loaded.errors(), 0, "{:?}", loaded.problems);
        let mut s = Session::new(Book::new(loaded.file));
        s.join(7, "Steve", Ok(Progress::default()));
        s.process(1);
        s
    }

    fn completed(s: &Session, quest: &str) -> bool {
        s.player(7).unwrap().progress.is_completed(quest)
    }

    #[test]
    fn crafts_count_for_open_craft_tasks_by_id_and_by_tag() {
        let mut s = work();
        let lines = s.summary(7);
        for line in [" - Bake: open (0/2 craft minecraft:bread)", " - Burn: open (0/2 craft #minecraft:logs)", " - Grow: open (0/3 experience levels)"] {
            assert!(lines.iter().any(|l| l == line), "{line} in {lines:?}");
        }
        s.crafted(7, "minecraft:bread", 1, &logs);
        s.crafted(7, "bread", 1, &logs);
        s.crafted(7, "minecraft:oak_log", 2, &logs);
        // Nothing asks for sticks, that is no item, and nobody is player 8.
        s.crafted(7, "minecraft:stick", 9, &logs);
        s.crafted(7, "Not An Item", 1, &logs);
        s.crafted(8, "minecraft:bread", 2, &logs);
        let effects = s.process(2);
        assert_eq!(effects.len(), 4, "{effects:?}");
        assert!(completed(&s, "w/bake") && completed(&s, "w/burn"));
        // Done: more bread reports nothing.
        s.crafted(7, "minecraft:bread", 1, &logs);
        assert!(s.process(3).is_empty());
    }

    #[test]
    fn levels_and_places_are_watched_while_a_task_waits_for_them() {
        let mut s = work();
        // An experience task waits, but the level is not known before it changes.
        assert!(!s.observe(7));
        assert!(s.process(2).is_empty());
        s.set_level(7, 2);
        s.process(3);
        assert_eq!(s.player(7).unwrap().progress.task("w/grow/t"), 2);
        // A level below zero is zero.
        s.set_level(7, -4);
        s.process(4);
        assert_eq!((s.player(7).unwrap().level, s.player(7).unwrap().progress.task("w/grow/t")), (Some(0), 0));
        s.set_level(7, 3);
        s.process(5);
        assert!(completed(&s, "w/grow"));

        // The next quest asks where they stand, in whole blocks.
        assert!(s.observe(7));
        s.at(7, "overworld", [4.9, 64.0, -0.5]);
        assert!(s.process(6).is_empty());
        s.at(7, "minecraft:overworld", [4.9, 64.0, 0.0]);
        s.process(7);
        assert!(completed(&s, "w/walk"));
        assert!(!s.observe(7));
        assert!(!s.observe(8));
    }

    #[test]
    fn a_level_heard_earlier_counts_when_its_task_opens() {
        let mut s = work();
        s.set_level(7, 9);
        s.process(2);
        assert!(completed(&s, "w/grow") && !completed(&s, "w/later"));
        s.crafted(7, "minecraft:birch_log", 2, &logs);
        s.process(3);
        assert!(completed(&s, "w/burn") && !completed(&s, "w/later"));
        // The level has not changed since: the next look reports it again.
        s.observe(7);
        s.process(4);
        assert!(completed(&s, "w/later"));
    }

    #[test]
    fn a_quest_waiting_to_start_over_is_woken_by_a_look() {
        let chapter = r#"{ "id": "r", "title": "R", "quests": [ { "id": "again", "title": "Again",
            "repeatable": true, "repeat_cooldown": 30, "tasks": [ { "id": "ok", "type": "checkmark" } ] } ] }"#;
        let loaded = load(None, &[("r.json", chapter)]);
        assert_eq!(loaded.errors(), 0, "{:?}", loaded.problems);
        let mut s = Session::new(Book::new(loaded.file));
        s.join(7, "Steve", Ok(Progress::default()));
        s.push(7, QuestEvent::Check { task: "r/again/ok".into() });
        s.process(100);
        assert!(completed(&s, "r/again"));
        s.observe(7);
        assert!(s.process(129).is_empty());
        // Time alone does nothing: the engine only runs on an event.
        assert!(s.process(131).is_empty());
        s.observe(7);
        assert_eq!(s.process(131), [(7, Effect::QuestStartedOver { quest: "r/again".into() })]);
        assert!(!completed(&s, "r/again"));
    }

    #[test]
    fn a_hand_in_plan_takes_plain_stacks_up_to_what_is_missing() {
        let wanted = |item: ItemMatch, missing| Wanted { task: "c/q/t".into(), item, missing, consume: true };
        let oak = || ItemMatch::Id("minecraft:oak_log".into());
        let any_log = || ItemMatch::Tag("minecraft:logs".into());
        let stacks = [
            ("minecraft:oak_log", 3, true),
            ("minecraft:bread", 5, true),
            ("minecraft:birch_log", 4, true),
            ("minecraft:oak_log", 2, true),
            ("minecraft:spruce_log", 9, false),
        ];
        let plan = |wanted: &Wanted| hand_in_plan(wanted, &stacks, &logs);
        // By id: every stack of the item, and no more than is missing.
        assert_eq!(plan(&wanted(oak(), 4)), [("minecraft:oak_log".to_owned(), 4)]);
        assert_eq!(plan(&wanted(oak(), 64)), [("minecraft:oak_log".to_owned(), 5)]);
        // By tag: the items as they are met, until there is enough. The
        // spruce is a named stack: left alone.
        assert_eq!(
            plan(&wanted(any_log(), 7)),
            [("minecraft:oak_log".to_owned(), 5), ("minecraft:birch_log".to_owned(), 2)]
        );
        assert_eq!(
            plan(&wanted(any_log(), 64)),
            [("minecraft:oak_log".to_owned(), 5), ("minecraft:birch_log".to_owned(), 4)]
        );
        assert!(plan(&wanted(ItemMatch::Id("minecraft:stone".into()), 3)).is_empty());
        assert!(plan(&wanted(oak(), 0)).is_empty());
        // An item also held in a stack that is not plain is left alone whole.
        let mixed = [("minecraft:bread", 5, true), ("minecraft:bread", 1, false)];
        assert!(hand_in_plan(&wanted(ItemMatch::Id("minecraft:bread".into()), 3), &mixed, &logs).is_empty());
    }

    #[test]
    fn console_lines_for_a_player_refuse_unsafe_text() {
        assert_eq!(clear_command("Steve_1", "bread", 3).as_deref(), Some("clear Steve_1 minecraft:bread 3"));
        assert_eq!(clear_command("a b", "bread", 3), None);
        assert_eq!(clear_command("Steve", "bread 1", 3), None);
        assert_eq!(clear_command("Steve", "bread", 0), None);

        assert_eq!(
            reward_command("/effect give {player} minecraft:speed 30", "Steve", 7).as_deref(),
            Some("effect give Steve minecraft:speed 30")
        );
        assert_eq!(reward_command("say {uuid}", "Steve", 7), Some(format!("say {}", player_id(7))));
        assert_eq!(reward_command("say hi", "Steve; stop", 7), None);
        assert_eq!(reward_command("  ", "Steve", 7), None);
        assert_eq!(reward_command("say a\nstop", "Steve", 7), None);
        assert!(safe_name("Alex_2") && !safe_name("") && !safe_name("seventeen_letters_"));
    }

    #[test]
    fn toasts_have_a_title_and_a_chat_line_for_clients_without_the_book() {
        assert_eq!(toast(Some("Welcome"), " Mind the lava. "), ("Welcome".to_owned(), "Mind the lava.".to_owned()));
        assert_eq!(toast(None, "Hi"), ("Quest reward".to_owned(), "Hi".to_owned()));
        assert_eq!(toast(Some("  "), ""), ("Quest reward".to_owned(), String::new()));
        assert_eq!(toast_line("Welcome", "Mind the lava."), "Welcome: Mind the lava.");
        assert_eq!(toast_line("Welcome", ""), "Welcome");
    }

    #[test]
    fn give_commands_refuse_unsafe_text() {
        assert_eq!(give_command("Steve_1", "bread", 3).as_deref(), Some("give Steve_1 minecraft:bread 3"));
        assert_eq!(give_command("a b", "bread", 3), None);
        assert_eq!(give_command("Steve", "bread 1; stop", 3), None);
        assert_eq!(give_command("Steve", "bread", 0), None);
    }
}
