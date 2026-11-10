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
use super::hq_progress::{self as engine, Effect, QuestEvent, Wanted};
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
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Session {
    pub fn new(book: Book) -> Self {
        Self {
            book,
            players: HashMap::new(),
            queue: Vec::new(),
            tick: 0,
            next_look: HashMap::new(),
            poked: HashSet::new(),
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
        self.players.insert(uuid, Player { name: name.to_owned(), progress, dirty: false, unsynced: false, saveable });
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

    /// One server tick went by: the players whose inventory to look at now.
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
            let outcome = engine::apply(&self.book, &mut player.progress, &player_id(uuid), now, &event);
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
        let player = self.players.get_mut(&uuid).ok_or("That player is not on.")?;
        let outcome = engine::force_complete(&self.book, &mut player.progress, &player_id(uuid), now, quest)
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
    let name_ok = (1..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let item = resource_id(item)?;
    (name_ok && count > 0).then(|| format!("give {name} {item} {count}"))
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

    #[test]
    fn give_commands_refuse_unsafe_text() {
        assert_eq!(give_command("Steve_1", "bread", 3).as_deref(), Some("give Steve_1 minecraft:bread 3"));
        assert_eq!(give_command("a b", "bread", 3), None);
        assert_eq!(give_command("Steve", "bread 1; stop", 3), None);
        assert_eq!(give_command("Steve", "bread", 0), None);
    }
}
