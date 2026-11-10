// The server's quest state while a world is open: the pack, the parties, the
// progress of every team with a member on (a lone player is their own team),
// the events waiting for the next tick, and when to look at each player's
// inventory. The glue feeds it hooks and inventories, calls
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
use super::hq_teams::{self as teams, Departure, Teams};
use std::collections::{HashMap, HashSet};

/// A player on the server.
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Player {
    pub name: String,
    /// The team whose progress they play on: their party's id, else their
    /// own player id (hq_teams).
    pub team: String,
    /// Their team's progress changed since it was last sent to their book.
    pub unsynced: bool,
    /// Their experience level, once a change of it was heard.
    pub level: Option<u32>,
    /// What their last inventory look counted for each item task: an item
    /// task counts the member of the team who holds the most.
    held: HashMap<String, u64>,
}

/// A team's progress while one of its members is on.
#[derive(Clone, Debug)]
struct Team {
    progress: Progress,
    /// Changed since it was last saved.
    dirty: bool,
    /// False when its file could not be read: it is never overwritten.
    saveable: bool,
}

/// What a team command says: to the player who ran it, and to others on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeamReply {
    pub lines: Vec<String>,
    pub others: Vec<(u128, String)>,
}

/// Reads a team's progress from its file (the glue's store), for a team none
/// of whose members is on.
pub type Load<'a> = &'a dyn Fn(&str) -> Result<Progress, String>;

const TEAMS_OFF: &str = "Parties are off in this world: its teams.json could not be read (see the log).";
const PARTIES_OFF: &str = "Parties are turned off on this server.";

/// What the server's settings (hq_config) ask of the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Parties may be started and joined (leaving always works).
    pub parties: bool,
    /// The most members a party may have; 0: no limit.
    pub party_size: u32,
    /// Ticks between two inventory looks; 0: the pack's `detection_delay`.
    pub detection_delay: u32,
}

impl Default for Rules {
    fn default() -> Self {
        Self { parties: true, party_size: 0, detection_delay: 0 }
    }
}
const NOT_ON: &str = "You are not in the quest book yet.";

#[derive(Debug)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Session {
    book: Book,
    players: HashMap<u128, Player>,
    /// Team id → its progress, for every team with a member on.
    teams: HashMap<String, Team>,
    /// Who is in which party (teams.json).
    parties: Teams,
    /// False when teams.json could not be read: parties cannot change, and
    /// the file is never overwritten.
    parties_ok: bool,
    /// The parties changed since they were last saved.
    parties_dirty: bool,
    /// Progress of teams that nobody plays on any more, still to save.
    unloaded: Vec<(String, Progress)>,
    queue: Vec<(u128, QuestEvent)>,
    tick: u64,
    /// The tick of each player's next inventory look.
    next_look: HashMap<u128, u64>,
    /// Players to look at on the next tick whatever their turn.
    poked: HashSet<u128>,
    /// The tick from which each player's level and advancements may be
    /// asked again: at most once per `detection_delay`, pokes or not.
    next_probe: HashMap<u128, u64>,
    /// The state of the random bits handed to the engine (random rewards).
    roll: u64,
    rules: Rules,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Session {
    pub fn new(book: Book) -> Self {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        Self {
            book,
            players: HashMap::new(),
            teams: HashMap::new(),
            parties: Teams::default(),
            parties_ok: true,
            parties_dirty: false,
            unloaded: Vec::new(),
            queue: Vec::new(),
            tick: 0,
            next_look: HashMap::new(),
            poked: HashSet::new(),
            next_probe: HashMap::new(),
            roll: seed,
            rules: Rules::default(),
        }
    }

    /// The server's settings, read again: they apply from the next tick.
    pub fn set_rules(&mut self, rules: Rules) {
        self.rules = rules;
    }

    /// Ticks between two looks at a player: the server's setting, else the
    /// pack's.
    fn delay(&self) -> u64 {
        let delay = if self.rules.detection_delay > 0 { self.rules.detection_delay } else { self.book.settings().detection_delay };
        u64::from(delay.max(1))
    }

    pub fn book(&self) -> &Book {
        &self.book
    }

    /// The world's parties as the store read them, before anyone joins. An
    /// Err leaves everyone on their own progress and turns party commands
    /// off, so the unreadable file stays for someone to repair.
    pub fn set_parties(&mut self, parties: Result<Teams, String>) {
        (self.parties, self.parties_ok) = match parties {
            Ok(parties) => (parties, true),
            Err(_) => (Teams::default(), false),
        };
        self.parties_dirty = false;
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

    /// The team a player plays on (on or not): the id of their progress file.
    pub fn team_of(&self, uuid: u128) -> String {
        self.parties.team_of(&player_id(uuid))
    }

    /// Whether a member of `team` is on: its progress is already in play.
    pub fn team_loaded(&self, team: &str) -> bool {
        self.teams.contains_key(team)
    }

    /// A player joined. `progress` is their team's as the store read it, and
    /// is not used when another member is on already. An Err keeps the team
    /// playing on an empty progress that is never saved, so the unreadable
    /// file stays for someone to repair.
    pub fn join(&mut self, uuid: u128, name: &str, progress: Result<Progress, String>) {
        let id = player_id(uuid);
        let team = self.parties.team_of(&id);
        if self.parties.saw(&id, name) {
            self.parties_dirty = true;
        }
        self.teams.entry(team.clone()).or_insert_with(|| match progress {
            Ok(progress) => Team { progress, dirty: false, saveable: true },
            Err(_) => Team { progress: Progress::default(), dirty: false, saveable: false },
        });
        let player = Player { name: name.to_owned(), team, unsynced: false, level: None, held: HashMap::new() };
        self.players.insert(uuid, player);
        self.push(uuid, QuestEvent::Refresh);
        self.poke(uuid);
    }

    /// Whether the player is due their first quest book: true once, then it
    /// is written in their team's progress and saved.
    pub fn take_book(&mut self, uuid: u128) -> bool {
        let Some(team) = self.players.get(&uuid).and_then(|p| self.teams.get_mut(&p.team)) else {
            return false;
        };
        if team.progress.book_given {
            return false;
        }
        team.progress.book_given = true;
        team.dirty = true;
        true
    }

    /// A player left: their team's progress to save now, if nobody else of
    /// it is on and it changed.
    pub fn leave(&mut self, uuid: u128) -> Option<(String, Progress)> {
        self.next_look.remove(&uuid);
        self.poked.remove(&uuid);
        self.next_probe.remove(&uuid);
        self.queue.retain(|(who, _)| *who != uuid);
        let player = self.players.remove(&uuid)?;
        if self.players.values().any(|p| p.team == player.team) {
            return None;
        }
        let team = self.teams.remove(&player.team)?;
        (team.dirty && team.saveable).then_some((player.team, team.progress))
    }

    pub fn player(&self, uuid: u128) -> Option<&Player> {
        self.players.get(&uuid)
    }

    /// The progress a player plays on: their team's.
    pub fn progress(&self, uuid: u128) -> Option<&Progress> {
        self.players.get(&uuid).and_then(|p| self.teams.get(&p.team)).map(|t| &t.progress)
    }

    /// The player on with this name, ignoring case.
    pub fn find_by_name(&self, name: &str) -> Option<u128> {
        self.players.iter().find(|(_, p)| p.name.eq_ignore_ascii_case(name)).map(|(uuid, _)| *uuid)
    }

    /// The members of `team` who are on, in order.
    fn members(&self, team: &str) -> Vec<u128> {
        let mut members: Vec<u128> = self.players.iter().filter(|(_, p)| p.team == team).map(|(uuid, _)| *uuid).collect();
        members.sort_unstable();
        members
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
        let Some(progress) = self.progress(uuid) else {
            return;
        };
        let Some(item) = resource_id(item) else {
            return;
        };
        for wanted in engine::wanted_crafts(&self.book, progress) {
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
        let Some(progress) = self.progress(uuid) else {
            return false;
        };
        let watch = engine::watch(&self.book, progress);
        let level = self.players.get(&uuid).and_then(|p| p.level);
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

    /// What to ask the server about the player's level and advancements:
    /// Horizon does not tell them as they change for the host of a
    /// single-player world, so the glue asks when it looks at them. Nothing
    /// when they were asked less than `detection_delay` ticks ago: a pickup
    /// pokes a look at the inventory, not a new question.
    pub fn take_probe(&mut self, uuid: u128) -> engine::Probe {
        if self.next_probe.get(&uuid).is_some_and(|&next| self.tick < next) {
            return engine::Probe::default();
        }
        let probe = self.progress(uuid).map(|p| engine::probe(&self.book, p)).unwrap_or_default();
        if probe != engine::Probe::default() {
            let delay = self.delay();
            self.next_probe.insert(uuid, self.tick + delay);
        }
        probe
    }

    /// The open tasks of the player a command can name for `verb`
    /// (`check`: checkmarks, `submit`: hand-ins): (full id, quest title).
    pub fn open_for(&self, uuid: u128, verb: Verb) -> Vec<(String, String)> {
        self.progress(uuid).map(|p| engine::open_tasks_where(&self.book, p, &|k| verb.takes(k))).unwrap_or_default()
    }

    /// The full id of the task the player typed for `verb` (a short id is
    /// enough when it names one open task).
    pub fn resolve(&self, uuid: u128, verb: Verb, typed: &str) -> String {
        match self.progress(uuid) {
            Some(p) => engine::resolve_task(&self.book, p, typed, &|k| verb.takes(k)),
            None => typed.trim().to_owned(),
        }
    }

    /// One server tick went by: the players to look at now (their
    /// inventory, where they stand). Each player has their own tick in every
    /// `delay`, taken from their UUID, so a server's players are not all
    /// looked at on the same tick; a poke looks at once without moving it.
    pub fn advance(&mut self) -> Vec<u128> {
        self.tick += 1;
        let delay = self.delay();
        let tick = self.tick;
        let mut due = Vec::new();
        for &uuid in self.players.keys() {
            let next = self.next_look.entry(uuid).or_insert(tick);
            if self.poked.contains(&uuid) || *next <= tick {
                let phase = (uuid % u128::from(delay)) as u64;
                *next = tick + 1 + (phase + delay - (tick + 1) % delay) % delay;
                due.push(uuid);
            }
        }
        self.poked.clear();
        due.sort_unstable();
        due
    }

    /// The item tasks this player can progress by holding items.
    pub fn wanted(&self, uuid: u128) -> Vec<Wanted> {
        let Some(progress) = self.progress(uuid) else {
            return Vec::new();
        };
        engine::wanted_items(&self.book, progress).into_iter().filter(|w| !w.consume).collect()
    }

    /// What a hand-in for `task` should take from this player.
    pub fn submit_request(&self, uuid: u128, task: &str) -> Result<Wanted, String> {
        let progress = self.progress(uuid).ok_or(NOT_ON)?;
        engine::submit_request(&self.book, progress, task).map_err(|r| r.text().to_owned())
    }

    /// What the team of a player reporting `event` makes of it: an item
    /// count or a level counts the member who has the most.
    fn for_team(&mut self, uuid: u128, event: QuestEvent) -> QuestEvent {
        let Some(team) = self.players.get(&uuid).map(|p| p.team.clone()) else {
            return event;
        };
        match event {
            QuestEvent::ItemCount { task, count } => {
                if let Some(player) = self.players.get_mut(&uuid) {
                    player.held.insert(task.clone(), count);
                }
                let count = self.players.values().filter(|p| p.team == team).filter_map(|p| p.held.get(&task)).copied().max().unwrap_or(count);
                QuestEvent::ItemCount { task, count }
            }
            QuestEvent::Level { level } => {
                let best = self.players.values().filter(|p| p.team == team).filter_map(|p| p.level).max().unwrap_or(level);
                QuestEvent::Level { level: best.max(level) }
            }
            other => other,
        }
    }

    /// Runs every waiting event through the engine, at `now` (Unix seconds):
    /// what to carry out, for whom. What a quest or a chapter did is told to
    /// every member on; a reward or a refusal only to the player concerned.
    pub fn process(&mut self, now: u64) -> Vec<(u128, Effect)> {
        let mut effects = Vec::new();
        let mut wake = Vec::new();
        for (uuid, event) in std::mem::take(&mut self.queue) {
            let event = self.for_team(uuid, event);
            let Some(team_id) = self.players.get(&uuid).map(|p| p.team.clone()) else {
                continue;
            };
            let members = self.members(&team_id);
            let roll = engine::next_roll(&mut self.roll);
            let Some(team) = self.teams.get_mut(&team_id) else {
                continue;
            };
            let id = player_id(uuid);
            let who = Actor::new(&id, now).with_roll(roll);
            let outcome = engine::apply(&self.book, &mut team.progress, &who, &event);
            if outcome.changed {
                team.dirty = true;
                self.mark_unsynced(&team_id);
            }
            for effect in outcome.effects {
                let shared = matches!(
                    effect,
                    Effect::QuestCompleted { .. } | Effect::ChapterCompleted { .. } | Effect::QuestStartedOver { .. }
                );
                if shared {
                    // The others may have automatic rewards due now.
                    wake.extend(members.iter().copied().filter(|&m| m != uuid));
                    effects.extend(members.iter().map(|&m| (m, effect.clone())));
                } else {
                    effects.push((uuid, effect));
                }
            }
        }
        wake.sort_unstable();
        wake.dedup();
        for uuid in wake {
            self.push(uuid, QuestEvent::Refresh);
        }
        effects
    }

    /// Every member on of `team` is sent its progress again.
    fn mark_unsynced(&mut self, team: &str) {
        for player in self.players.values_mut().filter(|p| p.team == team) {
            player.unsynced = true;
        }
    }

    /// The progress to write now: every changed one that may be saved, by
    /// team id, and those of teams nobody plays on any more.
    pub fn take_dirty(&mut self) -> Vec<(String, Progress)> {
        let mut out = std::mem::take(&mut self.unloaded);
        for (id, team) in &mut self.teams {
            if team.dirty && team.saveable {
                team.dirty = false;
                out.push((id.clone(), team.progress.clone()));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// The parties to write now, when they changed and may be saved.
    pub fn take_parties(&mut self) -> Option<Teams> {
        let changed = std::mem::take(&mut self.parties_dirty);
        (changed && self.parties_ok).then(|| self.parties.clone())
    }

    /// The progress to send to the books now: every one changed since it
    /// was last sent, with the id the player's claims are recorded under.
    pub fn take_unsynced(&mut self) -> Vec<(u128, String, Progress)> {
        let mut out = Vec::new();
        for (&uuid, player) in &mut self.players {
            if player.unsynced
                && let Some(team) = self.teams.get(&player.team)
            {
                player.unsynced = false;
                out.push((uuid, player_id(uuid), team.progress.clone()));
            }
        }
        out.sort_unstable_by_key(|(uuid, _, _)| *uuid);
        out
    }

    /// The team of a player on, for an admin command.
    fn team_mut(&mut self, uuid: u128) -> Result<(String, &mut Team), String> {
        let team = self.players.get(&uuid).map(|p| p.team.clone()).ok_or("That player is not on.")?;
        let found = self.teams.get_mut(&team).ok_or("That player is not on.")?;
        Ok((team, found))
    }

    /// `/quests complete`: what it did, or why not.
    pub fn force_complete(&mut self, uuid: u128, quest: &str, now: u64) -> Result<Vec<Effect>, String> {
        let roll = engine::next_roll(&mut self.roll);
        let book = self.book.clone();
        let (team_id, team) = self.team_mut(uuid)?;
        let id = player_id(uuid);
        let who = Actor::new(&id, now).with_roll(roll);
        let outcome =
            engine::force_complete(&book, &mut team.progress, &who, quest).ok_or_else(|| format!("There is no quest `{quest}`."))?;
        team.dirty |= outcome.changed;
        if outcome.changed {
            self.mark_unsynced(&team_id);
        }
        Ok(outcome.effects)
    }

    /// `/quests reset`: whether there was anything to reset.
    pub fn reset(&mut self, uuid: u128, quest: &str) -> Result<bool, String> {
        let book = self.book.clone();
        let (team_id, team) = self.team_mut(uuid)?;
        let changed = engine::reset_quest(&book, &mut team.progress, quest).ok_or_else(|| format!("There is no quest `{quest}`."))?;
        team.dirty |= changed;
        if changed {
            self.mark_unsynced(&team_id);
        }
        self.poke(uuid);
        Ok(changed)
    }

    /// `/quests reset_all`: the whole progress of the player's team.
    pub fn reset_all(&mut self, uuid: u128) -> Result<(), String> {
        let (team_id, team) = self.team_mut(uuid)?;
        let book_given = team.progress.book_given;
        team.progress = Progress { book_given, ..Progress::default() };
        team.dirty = true;
        self.mark_unsynced(&team_id);
        self.push(uuid, QuestEvent::Refresh);
        self.poke(uuid);
        Ok(())
    }

    /// `/quests progress`: the player's party, each chapter's tally, then
    /// the quests they can work on now or have rewards waiting in.
    pub fn summary(&self, uuid: u128) -> Vec<String> {
        let Some(progress) = self.progress(uuid) else {
            return vec![NOT_ON.to_owned()];
        };
        let book = &self.book;
        let statuses = book.statuses(progress, &player_id(uuid));
        let mut lines = Vec::new();
        if let Some(party) = self.parties.party_of(&player_id(uuid)).and_then(|id| self.parties.party(id)) {
            lines.push(format!("Party {}: its progress is shared by its {} members.", party.name, party.members.len()));
        }
        for c in 0..book.file().chapters.len() {
            let chapter = book.chapter(c);
            let (done, total) = book.chapter_tally(c, progress);
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
                    lines.push(format!(" - {}: {state}{}", label(&quest.title, &book.nodes()[n].id), tasks(book, progress, n)));
                }
            }
        }
        lines
    }

    /// `/quests team`: the player's party and invitations.
    pub fn team_info(&self, uuid: u128) -> Vec<String> {
        if !self.parties_ok {
            return vec![TEAMS_OFF.to_owned()];
        }
        let me = player_id(uuid);
        let mut lines = Vec::new();
        match self.parties.party_of(&me).and_then(|id| self.parties.party(id)) {
            Some(party) => {
                let on: HashSet<String> = self.players.keys().map(|&u| player_id(u)).collect();
                let names: Vec<String> = party
                    .members
                    .iter()
                    .map(|(id, name)| {
                        let mut shown = name.clone();
                        if *id == party.owner {
                            shown.push_str(" (owner)");
                        }
                        if !on.contains(id) {
                            shown.push_str(" (away)");
                        }
                        shown
                    })
                    .collect();
                lines.push(format!("Party {}: {}.", party.name, names.join(", ")));
                if !party.invites.is_empty() {
                    let invited: Vec<&str> = party.invites.values().map(String::as_str).collect();
                    lines.push(format!("Invited: {}.", invited.join(", ")));
                }
                lines.push("The party shares one progress. /quests team leave takes you back to your own.".to_owned());
            }
            None if !self.rules.parties => lines.push(format!("You play on your own progress. {PARTIES_OFF}")),
            None => {
                lines.push("You play on your own progress. Start a party with /quests team create <name>.".to_owned());
                for (_, name) in self.parties.invitations(&me) {
                    lines.push(format!("The party {name} invited you: /quests team join {name}"));
                }
            }
        }
        lines
    }

    /// The name of a player on, and their id.
    fn me(&self, uuid: u128) -> Result<(String, String), String> {
        if !self.parties_ok {
            return Err(TEAMS_OFF.to_owned());
        }
        let player = self.players.get(&uuid).ok_or(NOT_ON)?;
        Ok((player_id(uuid), player.name.clone()))
    }

    /// The player plays on `team` from now on, whose progress is in play.
    fn switch(&mut self, uuid: u128, team: &str) {
        let Some(player) = self.players.get_mut(&uuid) else {
            return;
        };
        let old = std::mem::replace(&mut player.team, team.to_owned());
        player.unsynced = true;
        player.held.clear();
        if !self.players.values().any(|p| p.team == old)
            && let Some(left) = self.teams.remove(&old)
            && left.dirty
            && left.saveable
        {
            self.unloaded.push((old, left.progress));
        }
        self.push(uuid, QuestEvent::Refresh);
        self.poke(uuid);
    }

    /// `/quests team create <name>`: the player's progress so far becomes
    /// the new party's.
    pub fn create_team(&mut self, uuid: u128, name: &str, now: u64) -> Result<TeamReply, String> {
        let (me, my_name) = self.me(uuid)?;
        self.parties_allowed()?;
        let roll = &mut self.roll;
        let party = self.parties.create(&me, &my_name, name, now, &mut || engine::next_roll(roll)).map_err(|r| r.text().to_owned())?;
        self.parties_dirty = true;
        let progress = self.progress(uuid).cloned().unwrap_or_default();
        self.teams.insert(party.clone(), Team { progress, dirty: true, saveable: true });
        self.switch(uuid, &party);
        let name = self.parties.party(&party).map(|p| p.name.clone()).unwrap_or_default();
        Ok(TeamReply {
            lines: vec![format!(
                "You started the party {name}: what you have done is its progress now. Invite players with /quests team invite <player>."
            )],
            others: Vec::new(),
        })
    }

    /// `/quests team invite <player>`: the owner invites a player who is on.
    pub fn invite(&mut self, uuid: u128, target: &str) -> Result<TeamReply, String> {
        let (me, my_name) = self.me(uuid)?;
        self.parties_allowed()?;
        let target_uuid = self.find_by_name(target).ok_or_else(|| format!("{target} is not on."))?;
        let target_name = self.players.get(&target_uuid).map(|p| p.name.clone()).unwrap_or_default();
        let max = self.party_size();
        let party = self.parties.invite(&me, &player_id(target_uuid), &target_name, max).map_err(|r| r.text().to_owned())?;
        self.parties_dirty = true;
        let name = self.parties.party(&party).map(|p| p.name.clone()).unwrap_or_default();
        Ok(TeamReply {
            lines: vec![format!("You invited {target_name} to the party {name}.")],
            others: vec![(target_uuid, format!("{my_name} invited you to the party {name}: /quests team join {name}"))],
        })
    }

    /// `/quests team join <name>`: the player accepts an invitation; what
    /// they have done is added to the party's progress.
    pub fn join_team(&mut self, uuid: u128, name: &str, load: Load<'_>) -> Result<TeamReply, String> {
        let (me, my_name) = self.me(uuid)?;
        self.parties_allowed()?;
        let max = self.party_size();
        let party = self.parties.can_join(&me, name, max).map_err(|r| r.text().to_owned())?;
        if !self.teams.contains_key(&party) {
            let progress = load(&party).map_err(|e| format!("The party's progress could not be read: {e}"))?;
            self.teams.insert(party.clone(), Team { progress, dirty: false, saveable: true });
        }
        self.parties.join(&me, &my_name, name, max).map_err(|r| r.text().to_owned())?;
        self.parties_dirty = true;
        let mine = self.progress(uuid).cloned().unwrap_or_default();
        if let Some(team) = self.teams.get_mut(&party) {
            teams::merge_progress(&mut team.progress, &mine);
            team.dirty = true;
        }
        let others = self.members(&party).into_iter().map(|m| (m, format!("{my_name} joined the party."))).collect();
        self.switch(uuid, &party);
        self.mark_unsynced(&party);
        let name = self.parties.party(&party).map(|p| p.name.clone()).unwrap_or_default();
        Ok(TeamReply {
            lines: vec![format!("You joined the party {name}: what you had done is added to its progress.")],
            others,
        })
    }

    /// `/quests team leave`: the player is back on their own progress, with
    /// the rewards they claimed in the party kept.
    pub fn leave_team(&mut self, uuid: u128, load: Load<'_>) -> Result<TeamReply, String> {
        let (me, my_name) = self.me(uuid)?;
        let name = self.party_name(&me);
        let departure = self.parties.leave(&me).map_err(|r| r.text().to_owned())?;
        self.parties_dirty = true;
        let mut lines = vec![format!(
            "You left the party {name}: you are back on your own progress, with the rewards you claimed kept."
        )];
        lines.extend(self.back_home(&me, Some(uuid), &departure.party, load));
        let others = self.departure_lines(&departure, &format!("{my_name} left the party."));
        Ok(TeamReply { lines, others })
    }

    /// `/quests team kick <player>`: the owner removes a member, on or not.
    pub fn kick(&mut self, uuid: u128, target: &str, load: Load<'_>) -> Result<TeamReply, String> {
        let (me, _) = self.me(uuid)?;
        let target_id = self.parties.member_named(&me, target).ok_or(teams::Refusal::NotMember.text())?;
        let name = self.party_name(&me);
        let target_name = self.parties.party_of(&me).and_then(|id| self.parties.party(id)).and_then(|p| p.members.get(&target_id).cloned()).unwrap_or_else(|| target.to_owned());
        let departure = self.parties.kick(&me, &target_id).map_err(|r| r.text().to_owned())?;
        self.parties_dirty = true;
        let online = self.players.keys().copied().find(|&u| player_id(u) == target_id);
        let mut lines = vec![format!("You removed {target_name} from the party.")];
        lines.extend(self.back_home(&target_id, online, &departure.party, load));
        let mut others = self.departure_lines(&departure, &format!("{target_name} was removed from the party."));
        others.retain(|(m, _)| *m != uuid);
        if let Some(target_uuid) = online {
            others.push((target_uuid, format!("You were removed from the party {name}: you are back on your own progress.")));
        }
        Ok(TeamReply { lines, others })
    }

    /// Err when the server turned parties off.
    fn parties_allowed(&self) -> Result<(), String> {
        if self.rules.parties { Ok(()) } else { Err(PARTIES_OFF.to_owned()) }
    }

    fn party_size(&self) -> usize {
        usize::try_from(self.rules.party_size).unwrap_or(usize::MAX)
    }

    fn party_name(&self, member: &str) -> String {
        self.parties.party_of(member).and_then(|id| self.parties.party(id)).map(|p| p.name.clone()).unwrap_or_default()
    }

    /// Member `member` (on as `online`, or away) is out of party `party`:
    /// back on their own progress, which keeps what they claimed in it.
    /// Lines for the one who asked when their file could not be read.
    fn back_home(&mut self, member: &str, online: Option<u128>, party: &str, load: Load<'_>) -> Vec<String> {
        let shared = self.teams.get(party).map(|t| t.progress.clone()).unwrap_or_default();
        let (personal, saveable, lines) = match load(member) {
            Ok(mut personal) => {
                teams::keep_claims(&mut personal, &shared, member);
                (personal, true, Vec::new())
            }
            Err(e) => (Progress::default(), false, vec![format!("Their own progress could not be read, so it will not be saved: {e}")]),
        };
        match online {
            Some(uuid) => {
                self.teams.insert(member.to_owned(), Team { progress: personal, dirty: true, saveable });
                self.switch(uuid, member);
            }
            None if saveable => self.unloaded.push((member.to_owned(), personal)),
            None => {}
        }
        lines
    }

    /// What the members still on are told when someone is out of the party.
    fn departure_lines(&self, departure: &Departure, line: &str) -> Vec<(u128, String)> {
        let mut out = Vec::new();
        for m in self.members(&departure.party) {
            out.push((m, line.to_owned()));
            if let Some(owner) = &departure.new_owner {
                let owner = self.parties.party(&departure.party).and_then(|p| p.members.get(owner).cloned()).unwrap_or_default();
                out.push((m, format!("{owner} owns the party now.")));
            }
        }
        out
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

/// The task commands that name a task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Check,
    Submit,
}

impl Verb {
    /// Whether a task of this kind is one the verb acts on.
    pub fn takes(self, kind: &TaskKind) -> bool {
        match self {
            Verb::Check => *kind == TaskKind::Checkmark,
            Verb::Submit => matches!(kind, TaskKind::Item { consume: true, .. }),
        }
    }
}

/// The console line that succeeds when player `name` has reached `level`
/// (vanilla's `level=` selector), and fails otherwise.
pub fn level_probe_command(name: &str, level: u32) -> Option<String> {
    safe_name(name).then(|| format!("execute if entity @a[name={name},level={level}..]"))
}

/// The console line that succeeds when player `name` has done
/// `advancement` (vanilla's `advancements=` selector), and fails otherwise.
pub fn advancement_probe_command(name: &str, advancement: &str) -> Option<String> {
    let advancement = resource_id(advancement)?;
    safe_name(name).then(|| format!("execute if entity @a[name={name},advancements={{{advancement}=true}}]"))
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
        // Player 7's own tick in every 10 is the 7th: 7, 17, 27...
        let due: Vec<u64> = (2..=30).filter(|_| !s.advance().is_empty()).collect();
        assert_eq!(due, [7, 17, 27]);
        s.poke(7);
        assert_eq!(s.advance(), [7]); // tick 31: poked
        // The poke does not move their tick.
        let due: Vec<u64> = (32..=40).filter(|_| !s.advance().is_empty()).collect();
        assert_eq!(due, [37]);
    }

    #[test]
    fn players_are_looked_at_on_ticks_of_their_own_and_the_server_can_slow_it() {
        let mut s = session();
        for uuid in 0..10 {
            s.join(uuid, "P", Ok(Progress::default()));
        }
        assert_eq!(s.advance().len(), 10); // tick 1: everyone just joined
        for _ in 2..=10 {
            assert_eq!(s.advance().len(), 1, "one player a tick");
        }
        s.set_rules(Rules { detection_delay: 100, ..Rules::default() });
        s.advance();
        let looks: usize = (0..200).map(|_| s.advance().len()).sum();
        // The look each was already due, then two each in 200 ticks (not 20).
        assert!((20..=30).contains(&looks), "{looks}");
    }

    #[test]
    fn the_server_can_turn_parties_off_and_cap_them() {
        let mut s = two();
        s.join(9, "Kim", Ok(Progress::default()));
        s.set_rules(Rules { parties: false, ..Rules::default() });
        assert_eq!(s.create_team(7, "Miners", 1), Err(PARTIES_OFF.to_owned()));
        assert_eq!(s.team_info(7), [format!("You play on your own progress. {PARTIES_OFF}")]);
        s.set_rules(Rules { party_size: 2, ..Rules::default() });
        s.create_team(7, "Miners", 1).unwrap();
        s.invite(7, "Alex").unwrap();
        s.invite(7, "Kim").unwrap();
        s.join_team(8, "Miners", &no_file).unwrap();
        assert_eq!(s.join_team(9, "Miners", &no_file), Err("That party is full.".to_owned()));
        // Off again: no joining, but leaving still works.
        s.set_rules(Rules { parties: false, ..Rules::default() });
        assert_eq!(s.join_team(9, "Miners", &no_file), Err(PARTIES_OFF.to_owned()));
        assert!(s.leave_team(8, &no_file).is_ok());
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
        assert!(s.progress(7).unwrap().is_completed("c/zombie"));
        // example_chapter = false: an empty pack, refreshed for everyone.
        let full = s.book().clone();
        s.set_book(Book::new(load(None, &[]).file));
        s.process(2);
        assert!(s.progress(7).unwrap().is_completed("c/zombie"));
        s.set_book(full);
        s.process(3);
        assert!(s.progress(7).unwrap().is_completed("c/zombie"));
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
        s.progress(7).unwrap().is_completed(quest)
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
        assert_eq!(s.progress(7).unwrap().task("w/grow/t"), 2);
        // A level below zero is zero.
        s.set_level(7, -4);
        s.process(4);
        assert_eq!((s.player(7).unwrap().level, s.progress(7).unwrap().task("w/grow/t")), (Some(0), 0));
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
    fn the_first_book_is_given_once() {
        let mut s = work();
        assert!(s.take_book(7));
        assert!(!s.take_book(7));
        assert!(!s.take_book(8));
        // Written in the progress, so it is saved and kept on the next join.
        let saved = s.take_dirty();
        assert!(saved.iter().any(|(team, p)| *team == player_id(7) && p.book_given), "{saved:?}");
        let progress = s.leave(7).map_or_else(|| saved[0].1.clone(), |(_, p)| p);
        s.join(7, "Steve", Ok(progress));
        assert!(!s.take_book(7));
    }

    #[test]
    fn a_probe_and_short_ids_follow_the_open_tasks() {
        let mut s = work();
        s.advance();
        assert_eq!(s.take_probe(7).level, Some(3));
        assert_eq!(s.take_probe(8), engine::Probe::default());
        // Asked again before the delay (a pickup's poke): nothing to ask.
        s.poke(7);
        s.advance();
        assert_eq!(s.take_probe(7), engine::Probe::default());
        // The default delay is 20 ticks: asked again on the 21st.
        for _ in 0..19 {
            s.advance();
        }
        assert_eq!(s.take_probe(7).level, Some(3));
        // What a probe of the level found counts as a level heard.
        s.set_level(7, 3);
        s.process(2);
        assert!(completed(&s, "w/grow"));
        for _ in 0..20 {
            s.advance();
        }
        assert_eq!(s.take_probe(7).level, None);
        assert!(s.open_for(7, Verb::Check).is_empty());
        assert_eq!(s.resolve(7, Verb::Check, " nothing "), "nothing");
        assert_eq!(s.resolve(8, Verb::Submit, "x"), "x");
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

        assert_eq!(level_probe_command("Steve_1", 5).as_deref(), Some("execute if entity @a[name=Steve_1,level=5..]"));
        assert_eq!(level_probe_command("@a", 5), None);
        assert_eq!(
            advancement_probe_command("Steve", "story/mine_stone").as_deref(),
            Some("execute if entity @a[name=Steve,advancements={minecraft:story/mine_stone=true}]")
        );
        assert_eq!(advancement_probe_command("Steve", "story/mine stone"), None);
        assert_eq!(advancement_probe_command("a,b", "story/mine_stone"), None);

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

    /// Steve (7) and Alex (8) on, each on their own progress.
    fn two() -> Session {
        let mut s = session();
        s.set_parties(Ok(Teams::default()));
        s.join(7, "Steve", Ok(Progress::default()));
        s.join(8, "Alex", Ok(Progress::default()));
        s.process(1);
        s
    }

    fn no_file(_: &str) -> Result<Progress, String> {
        Ok(Progress::default())
    }

    fn zombie(s: &mut Session, uuid: u128) {
        s.push(uuid, QuestEvent::Killed { entity: "minecraft:zombie".into() });
    }

    #[test]
    fn a_party_starts_with_its_creators_progress_and_merges_who_joins() {
        let mut s = two();
        zombie(&mut s, 7);
        s.process(2);
        assert_eq!(s.progress(7).unwrap().task("c/zombie/t"), 1);
        let reply = s.create_team(7, "Miners", 3).unwrap();
        assert!(reply.lines[0].starts_with("You started the party Miners"), "{reply:?}");
        let party = s.team_of(7);
        assert!(party.starts_with("party-"));
        assert_eq!(s.progress(7).unwrap().task("c/zombie/t"), 1);

        // Alex's own progress joins the party's.
        s.push(8, QuestEvent::ItemCount { task: "c/logs/t".into(), count: 2 });
        s.process(4);
        assert_eq!(s.join_team(8, "miners", &no_file), Err("That party has not invited you.".to_owned()));
        let invite = s.invite(7, "alex").unwrap();
        assert_eq!(invite.others, [(8, "Steve invited you to the party Miners: /quests team join Miners".to_owned())]);
        assert!(s.team_info(8).iter().any(|l| l == "The party Miners invited you: /quests team join Miners"));
        let joined = s.join_team(8, "miners", &no_file).unwrap();
        assert_eq!(joined.others, [(7, "Alex joined the party.".to_owned())]);
        assert_eq!(s.team_of(8), party);
        let shared = s.progress(8).unwrap();
        assert_eq!((shared.task("c/zombie/t"), shared.task("c/logs/t")), (1, 2));
        assert_eq!(s.team_info(7)[0], "Party Miners: Steve (owner), Alex.");

        // Alex's own progress is put away (saved: it changed), and the party's waits to be written.
        let saved = s.take_dirty();
        let teams: Vec<&str> = saved.iter().map(|(t, _)| t.as_str()).collect();
        assert!(teams.contains(&party.as_str()) && teams.contains(&player_id(8).as_str()), "{teams:?}");
        assert!(s.take_parties().is_some());
        assert!(s.take_parties().is_none());
    }

    #[test]
    fn what_a_member_does_counts_for_the_party_and_rewards_stay_per_player() {
        let mut s = two();
        s.create_team(7, "Miners", 1).unwrap();
        s.invite(7, "Alex").unwrap();
        s.join_team(8, "Miners", &no_file).unwrap();
        s.process(2);
        s.take_unsynced();

        // Alex kills both zombies: the quest is done for Steve too, and both hear it.
        zombie(&mut s, 8);
        zombie(&mut s, 8);
        let effects = s.process(3);
        let done = Effect::QuestCompleted { quest: "c/zombie".into() };
        assert!(effects.contains(&(7, done.clone())) && effects.contains(&(8, done)), "{effects:?}");
        assert_eq!(s.take_unsynced().iter().map(|(u, _, _)| *u).collect::<Vec<_>>(), [7, 8]);

        // An item task counts the member who holds the most.
        s.push(7, QuestEvent::ItemCount { task: "c/logs/t".into(), count: 3 });
        s.push(8, QuestEvent::ItemCount { task: "c/logs/t".into(), count: 1 });
        s.process(4);
        assert_eq!(s.progress(7).unwrap().task("c/logs/t"), 3);
        s.push(8, QuestEvent::ItemCount { task: "c/logs/t".into(), count: 4 });
        s.process(5);
        assert!(s.progress(7).unwrap().is_completed("c/logs"));

        // Each member claims the reward for themselves.
        s.push(7, QuestEvent::Claim { reward: "c/logs/r".into(), choice: None });
        let effects = s.process(6);
        assert!(effects.iter().any(|(u, e)| *u == 7 && matches!(e, Effect::Reward { .. })), "{effects:?}");
        assert!(!effects.iter().any(|(u, _)| *u == 8));
        s.push(7, QuestEvent::Claim { reward: "c/logs/r".into(), choice: None });
        s.push(8, QuestEvent::Claim { reward: "c/logs/r".into(), choice: None });
        let effects = s.process(7);
        assert!(effects.contains(&(7, Effect::Refused(engine::Refusal::AlreadyClaimed))));
        assert!(effects.iter().any(|(u, e)| *u == 8 && matches!(e, Effect::Reward { .. })), "{effects:?}");
    }

    #[test]
    fn levels_count_the_highest_member() {
        let chapter = r#"{ "id": "l", "title": "L", "quests": [ { "id": "grow", "title": "Grow",
            "tasks": [ { "id": "t", "type": "xp_levels", "levels": 10 } ] } ] }"#;
        let loaded = load(None, &[("l.json", chapter)]);
        let mut s = Session::new(Book::new(loaded.file));
        s.join(7, "Steve", Ok(Progress::default()));
        s.join(8, "Alex", Ok(Progress::default()));
        s.create_team(7, "Miners", 1).unwrap();
        s.invite(7, "Alex").unwrap();
        s.join_team(8, "Miners", &no_file).unwrap();
        s.set_level(7, 6);
        s.set_level(8, 2);
        s.process(2);
        assert_eq!(s.progress(8).unwrap().task("l/grow/t"), 6);
    }

    #[test]
    fn leaving_gives_back_the_own_progress_with_its_claims() {
        let mut s = two();
        // Steve's own file, as the store would read it after he joined a party.
        let mut own = Progress::default();
        own.tasks.insert("c/logs/t".into(), 1);
        let files = |team: &str| if team == player_id(7) { Ok(own.clone()) } else { Ok(Progress::default()) };

        s.create_team(8, "Miners", 1).unwrap();
        s.invite(8, "Steve").unwrap();
        s.join_team(7, "Miners", &files).unwrap();
        s.push(7, QuestEvent::ItemCount { task: "c/logs/t".into(), count: 4 });
        s.process(2);
        s.push(7, QuestEvent::ClaimAll);
        s.process(3);
        let reply = s.leave_team(7, &files).unwrap();
        assert!(reply.lines[0].starts_with("You left the party Miners"), "{reply:?}");
        assert_eq!(reply.others, [(8, "Steve left the party.".to_owned())]);
        assert_eq!(s.team_of(7), player_id(7));
        let back = s.progress(7).unwrap();
        // His own tasks, not the party's, and the reward he claimed there.
        assert_eq!(back.task("c/logs/t"), 1);
        assert!(!back.is_completed("c/logs"));
        assert!(back.has_claimed("c/logs/r", &player_id(7)));
        // Alex keeps the party and its progress.
        assert!(s.progress(8).unwrap().is_completed("c/logs"));
        assert_eq!(s.leave_team(7, &files), Err("You are not in a party.".to_owned()));
    }

    #[test]
    fn the_last_member_out_puts_the_party_away_and_the_owner_passes_on() {
        let mut s = two();
        s.create_team(7, "Miners", 1).unwrap();
        s.invite(7, "Alex").unwrap();
        s.join_team(8, "Miners", &no_file).unwrap();
        let party = s.team_of(7);
        s.take_dirty();
        let reply = s.leave_team(7, &no_file).unwrap();
        assert_eq!(reply.others, [(8, "Steve left the party.".to_owned()), (8, "Alex owns the party now.".to_owned())]);
        zombie(&mut s, 8);
        s.process(2);
        s.leave_team(8, &no_file).unwrap();
        assert!(!s.team_loaded(&party));
        assert!(s.take_dirty().iter().any(|(t, p)| *t == party && p.task("c/zombie/t") == 1));
        assert_eq!(s.team_info(8)[0], "You play on your own progress. Start a party with /quests team create <name>.");
    }

    #[test]
    fn a_party_of_members_away_is_read_from_its_file() {
        let mut s = two();
        s.create_team(7, "Miners", 1).unwrap();
        s.invite(7, "Alex").unwrap();
        let party = s.team_of(7);
        zombie(&mut s, 7);
        s.process(2);
        let (team, saved) = s.leave(7).unwrap();
        assert_eq!(team, party);
        // Alex joins while nobody of the party is on.
        let file = |team: &str| if team == party { Ok(saved.clone()) } else { Err("no".to_owned()) };
        s.join_team(8, "Miners", &file).unwrap();
        assert_eq!(s.progress(8).unwrap().task("c/zombie/t"), 1);
        // Steve comes back on the party's progress, already in play.
        assert_eq!(s.team_of(7), party);
        s.join(7, "Steve", Err("not read".into()));
        assert_eq!(s.progress(7).unwrap().task("c/zombie/t"), 1);
        assert_eq!(s.team_info(8)[0], "Party Miners: Steve (owner), Alex.");
    }

    #[test]
    fn the_owner_kicks_a_member_on_or_away() {
        let mut s = two();
        s.join(9, "Kim", Ok(Progress::default()));
        s.create_team(7, "Miners", 1).unwrap();
        for name in ["Alex", "Kim"] {
            s.invite(7, name).unwrap();
        }
        s.join_team(8, "Miners", &no_file).unwrap();
        s.join_team(9, "Miners", &no_file).unwrap();
        s.push(9, QuestEvent::ItemCount { task: "c/logs/t".into(), count: 4 });
        s.process(2);
        s.push(9, QuestEvent::ClaimAll);
        s.process(3);
        assert_eq!(s.kick(8, "Kim", &no_file), Err("Only the party's owner can do that.".to_owned()));
        assert_eq!(s.kick(7, "Nobody", &no_file), Err("That player is not in your party.".to_owned()));

        // Kim is away: their own file gets the claim, to be saved.
        s.leave(9);
        s.take_dirty();
        let reply = s.kick(7, "kim", &no_file).unwrap();
        assert_eq!(reply.lines[0], "You removed Kim from the party.");
        assert_eq!(reply.others, [(8, "Kim was removed from the party.".to_owned())]);
        let saved = s.take_dirty();
        assert!(saved.iter().any(|(t, p)| *t == player_id(9) && p.has_claimed("c/logs/r", &player_id(9))), "{saved:?}");

        // Alex is on: back on their own progress at once, and told.
        let reply = s.kick(7, "Alex", &no_file).unwrap();
        assert!(reply.others.contains(&(8, "You were removed from the party Miners: you are back on your own progress.".to_owned())));
        assert_eq!(s.team_of(8), player_id(8));
    }

    #[test]
    fn parties_stay_off_when_their_file_is_unreadable() {
        let mut s = session();
        s.set_parties(Err("broken".into()));
        s.join(7, "Steve", Ok(Progress::default()));
        assert_eq!(s.team_info(7), [TEAMS_OFF]);
        assert_eq!(s.create_team(7, "Miners", 1), Err(TEAMS_OFF.to_owned()));
        assert!(s.take_parties().is_none());
        assert_eq!(s.team_of(7), player_id(7));
    }

    #[test]
    fn give_commands_refuse_unsafe_text() {
        assert_eq!(give_command("Steve_1", "bread", 3).as_deref(), Some("give Steve_1 minecraft:bread 3"));
        assert_eq!(give_command("a b", "bread", 3), None);
        assert_eq!(give_command("Steve", "bread 1; stop", 3), None);
        assert_eq!(give_command("Steve", "bread", 0), None);
    }
}
