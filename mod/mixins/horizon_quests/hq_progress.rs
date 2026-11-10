// The quest engine: what something a player did changes in their team's
// progress, and what must happen in the game because of it. The glue turns
// hooks and book requests into `QuestEvent`s, calls `apply` on the server
// tick, saves when the outcome says `changed`, and carries out the `Effect`s
// once its lock is released. Pure: no game types.
//
// The engine never looks at the game. For item tasks the glue asks
// `wanted_items`, counts them in the inventory and reports `ItemCount`; for
// hand-in tasks it asks `submit_request`, takes the items and reports
// `Submitted`; for craft tasks it asks `wanted_crafts` and reports `Crafted`.
// Where the player stands, their level and their advancements come in as
// `At`, `Level` and `Advancement`.
use super::hq_logic::Book;
use super::hq_model::{AutoClaim, ItemMatch, Progress, RewardKind, RewardTable, TableEntry, TaskKind, resource_id};

/// Who an event comes from, and when.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub struct Actor<'a> {
    /// The id this player's claims are kept under.
    pub player: &'a str,
    /// Unix seconds.
    pub now: u64,
    /// Random bits for what the event draws (random rewards).
    pub roll: u64,
}

#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
impl<'a> Actor<'a> {
    pub fn new(player: &'a str, now: u64) -> Self {
        Self { player, now, roll: 0 }
    }

    pub fn with_roll(self, roll: u64) -> Self {
        Self { roll, ..self }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum QuestEvent {
    /// The player holds `count` of what item task `task` asks for.
    ItemCount { task: String, count: u64 },
    /// `count` items were taken from the player for hand-in task `task`.
    Submitted { task: String, count: u64 },
    /// The player killed an entity of this type.
    Killed { entity: String },
    /// The player crafted `count` of what craft task `task` asks for.
    Crafted { task: String, count: u64 },
    /// The player's experience level is `level`.
    Level { level: u32 },
    /// The player stands on this block of this dimension.
    At { dimension: String, position: [i32; 3] },
    /// The player completed this advancement.
    Advancement { advancement: String },
    /// The player ticked checkmark task `task` in the book.
    Check { task: String },
    /// The player asks for reward `reward`; `choice` is the entry they
    /// picked when it is a choice reward.
    Claim { reward: String, choice: Option<u32> },
    /// The player asks for every reward waiting for them.
    ClaimAll,
    /// Nothing was done, but the pack, the progress or the time may be new
    /// (login, reload, a cooldown): complete what is due.
    Refresh,
}

/// Why a request from the book was not carried out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Refusal {
    UnknownTask,
    /// A checkmark asked of a task that is not one, or a hand-in of a task
    /// that takes none.
    WrongTaskType,
    AlreadyDone,
    /// The quest's dependencies are not met.
    NotOpen,
    /// The quest's tasks go in order and one before this is not done.
    NotYet,
    UnknownReward,
    NotCompleted,
    AlreadyClaimed,
    NothingToClaim,
    /// A choice reward asked for without saying which entry.
    NeedsChoice,
    /// A choice that is not an entry of the reward's table.
    BadChoice,
    /// The reward's table is missing or gives nothing.
    NoTable,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Refusal {
    /// What to tell the player.
    pub fn text(self) -> &'static str {
        match self {
            Refusal::UnknownTask => "That task does not exist.",
            Refusal::WrongTaskType => "That task cannot be done this way.",
            Refusal::AlreadyDone => "That task is already done.",
            Refusal::NotOpen => "That quest is not open yet.",
            Refusal::NotYet => "Do the tasks before that one first.",
            Refusal::UnknownReward => "That reward does not exist.",
            Refusal::NotCompleted => "That quest is not completed yet.",
            Refusal::AlreadyClaimed => "You already claimed that reward.",
            Refusal::NothingToClaim => "There is nothing to claim.",
            Refusal::NeedsChoice => "Pick that reward in the quest book: it lets you choose.",
            Refusal::BadChoice => "That is not one of the reward's choices.",
            Refusal::NoTable => "That reward has nothing to give.",
        }
    }
}

/// Something for the glue to do, or to tell the player. Ids are full ids.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Effect {
    TaskCompleted { task: String },
    QuestCompleted { quest: String },
    /// Every quest of the chapter that counts is completed.
    ChapterCompleted { chapter: String },
    /// Give the player this reward. It is already recorded as claimed, so a
    /// crash cannot give it twice. `kind` is what to give and `title` what
    /// the pack calls it: for a random or a choice reward, those of the
    /// table entry that came out.
    Reward { reward: String, title: Option<String>, kind: RewardKind },
    /// A repeatable quest started over.
    QuestStartedOver { quest: String },
    Refused(Refusal),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Outcome {
    /// Whether the progress changed: save it and send it to the book.
    pub changed: bool,
    pub effects: Vec<Effect>,
}

/// An item or craft task that can progress, and how many items it still
/// lacks.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Wanted {
    pub task: String,
    pub item: ItemMatch,
    pub missing: u64,
    /// Whether the items are handed in (taken on request) or only held.
    pub consume: bool,
}

/// Every task that counts right now and is not done: (node, task index).
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
fn open_tasks(book: &Book, progress: &Progress) -> Vec<(usize, usize)> {
    let mut open = Vec::new();
    for n in 0..book.nodes().len() {
        if !book.can_progress(n, progress) {
            continue;
        }
        for (t, task) in book.quest(n).tasks.iter().enumerate() {
            if progress.task(&book.task_id(n, t)) < task.kind.goal() && book.task_counts(n, t, progress) {
                open.push((n, t));
            }
        }
    }
    open
}

/// Every item task that can progress right now.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn wanted_items(book: &Book, progress: &Progress) -> Vec<Wanted> {
    let mut wanted = Vec::new();
    for (n, t) in open_tasks(book, progress) {
        let kind = &book.quest(n).tasks[t].kind;
        if let TaskKind::Item { item, consume, .. } = kind {
            let id = book.task_id(n, t);
            let missing = kind.goal().saturating_sub(progress.task(&id));
            wanted.push(Wanted { task: id, item: item.clone(), missing, consume: *consume });
        }
    }
    wanted
}

/// Every craft task that can progress right now.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn wanted_crafts(book: &Book, progress: &Progress) -> Vec<Wanted> {
    let mut wanted = Vec::new();
    for (n, t) in open_tasks(book, progress) {
        let kind = &book.quest(n).tasks[t].kind;
        if let TaskKind::Craft { item, .. } = kind {
            let id = book.task_id(n, t);
            let missing = kind.goal().saturating_sub(progress.task(&id));
            wanted.push(Wanted { task: id, item: item.clone(), missing, consume: false });
        }
    }
    wanted
}

/// What the glue should watch for a team, beyond its items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub struct Watch {
    /// A dimension or location task can progress: report `At`.
    pub place: bool,
    /// An experience task can progress: report `Level`.
    pub level: bool,
    /// A repeatable quest waits to start over: send `Refresh` now and then.
    pub repeat: bool,
}

#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn watch(book: &Book, progress: &Progress) -> Watch {
    let mut watch = Watch::default();
    for (n, t) in open_tasks(book, progress) {
        match book.quest(n).tasks[t].kind {
            TaskKind::Dimension { .. } | TaskKind::Location { .. } => watch.place = true,
            TaskKind::XpLevels { .. } => watch.level = true,
            _ => {}
        }
    }
    watch.repeat =
        (0..book.nodes().len()).any(|n| book.quest(n).repeatable && progress.is_completed(&book.nodes()[n].id));
    watch
}

/// The player asks to hand items in for `task`: what to take from them (at
/// most `missing`), then report with `Submitted`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn submit_request(book: &Book, progress: &Progress, task: &str) -> Result<Wanted, Refusal> {
    let (n, t) = open_task(book, progress, task)?;
    let kind = &book.quest(n).tasks[t].kind;
    match kind {
        TaskKind::Item { item, consume: true, .. } => Ok(Wanted {
            task: task.to_owned(),
            item: item.clone(),
            missing: kind.goal().saturating_sub(progress.task(task)),
            consume: true,
        }),
        _ => Err(Refusal::WrongTaskType),
    }
}

/// The next random bits after `state` (SplitMix64): the same generator for
/// the session's events and for the draws inside one event.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn next_roll(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The entry `roll` draws from `table`, by weight. None when the table can
/// give nothing.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn draw(table: &RewardTable, roll: u64) -> Option<&TableEntry> {
    let total: u64 = table.entries.iter().map(|entry| u64::from(entry.weight)).sum();
    if total == 0 {
        return None;
    }
    let mut pick = roll % total;
    table.entries.iter().find(|entry| {
        let weight = u64::from(entry.weight);
        if pick < weight {
            return true;
        }
        pick -= weight;
        false
    })
}

/// What `event`, from `who`, does to their team's `progress`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn apply(book: &Book, progress: &mut Progress, who: &Actor<'_>, event: &QuestEvent) -> Outcome {
    let mut out = Outcome::default();
    let mut roll = who.roll;
    let now = who.now;
    match event {
        QuestEvent::ItemCount { task, count } => {
            if let Some((n, t)) = book.find_task(task)
                && book.task_counts(n, t, progress)
                && matches!(book.quest(n).tasks[t].kind, TaskKind::Item { consume: false, .. })
            {
                set_task(book, progress, n, t, *count, now, &mut out);
            }
        }
        QuestEvent::Submitted { task, count } => {
            if let Some((n, t)) = book.find_task(task)
                && book.task_counts(n, t, progress)
                && matches!(book.quest(n).tasks[t].kind, TaskKind::Item { consume: true, .. })
            {
                let value = progress.task(task).saturating_add(*count);
                set_task(book, progress, n, t, value, now, &mut out);
            }
        }
        QuestEvent::Crafted { task, count } => {
            if let Some((n, t)) = book.find_task(task)
                && book.task_counts(n, t, progress)
                && matches!(book.quest(n).tasks[t].kind, TaskKind::Craft { .. })
            {
                let value = progress.task(task).saturating_add(*count);
                set_task(book, progress, n, t, value, now, &mut out);
            }
        }
        QuestEvent::Killed { entity } => {
            if let Some(killed) = resource_id(entity) {
                // Decided before counting: a quest or a task this kill opens
                // does not count it.
                for (n, t) in open_tasks(book, progress) {
                    if matches!(&book.quest(n).tasks[t].kind, TaskKind::Kill { entity, .. } if *entity == killed) {
                        let value = progress.task(&book.task_id(n, t)).saturating_add(1);
                        set_task(book, progress, n, t, value, now, &mut out);
                    }
                }
            }
        }
        QuestEvent::Level { level } => {
            for (n, t) in open_tasks(book, progress) {
                if matches!(book.quest(n).tasks[t].kind, TaskKind::XpLevels { .. }) {
                    set_task(book, progress, n, t, u64::from(*level), now, &mut out);
                }
            }
        }
        QuestEvent::At { dimension, position } => {
            for (n, t) in open_tasks(book, progress) {
                let there = match &book.quest(n).tasks[t].kind {
                    TaskKind::Dimension { dimension: wanted } => wanted == dimension,
                    TaskKind::Location { dimension: wanted, from, to } => {
                        wanted.as_ref().is_none_or(|wanted| wanted == dimension)
                            && (0..3).all(|axis| {
                                let (low, high) = (from[axis].min(to[axis]), from[axis].max(to[axis]));
                                (low..=high).contains(&position[axis])
                            })
                    }
                    _ => false,
                };
                if there {
                    set_task(book, progress, n, t, 1, now, &mut out);
                }
            }
        }
        QuestEvent::Advancement { advancement } => {
            // Remembered when a task of the pack names it, open or not:
            // `settle` ticks the tasks, now or when their quest opens.
            if let Some(id) = resource_id(advancement)
                && names_advancement(book, &id)
                && progress.advancements.insert(id)
            {
                out.changed = true;
            }
        }
        QuestEvent::Check { task } => match open_task(book, progress, task) {
            Ok((n, t)) if book.quest(n).tasks[t].kind == TaskKind::Checkmark => {
                set_task(book, progress, n, t, 1, now, &mut out);
            }
            Ok(_) => out.effects.push(Effect::Refused(Refusal::WrongTaskType)),
            Err(refusal) => out.effects.push(Effect::Refused(refusal)),
        },
        QuestEvent::Claim { reward, choice } => {
            let refusal = match book.find_reward(reward) {
                None => Some(Refusal::UnknownReward),
                Some((n, _)) if !progress.is_completed(&book.nodes()[n].id) => Some(Refusal::NotCompleted),
                Some(_) if progress.has_claimed(reward, who.player) => Some(Refusal::AlreadyClaimed),
                Some((n, r)) => claim(book, progress, who.player, n, r, *choice, &mut roll, &mut out).err(),
            };
            out.effects.extend(refusal.map(Effect::Refused));
        }
        QuestEvent::ClaimAll => {
            let (mut claimed, mut choices) = (0, 0);
            for n in 0..book.nodes().len() {
                if !progress.is_completed(&book.nodes()[n].id) {
                    continue;
                }
                for (r, reward) in book.quest(n).rewards.iter().enumerate() {
                    if progress.has_claimed(&book.reward_id(n, r), who.player) {
                        continue;
                    }
                    if matches!(reward.kind, RewardKind::Choice { .. }) {
                        // The book asks which entry; a command cannot.
                        choices += 1;
                    } else if claim(book, progress, who.player, n, r, None, &mut roll, &mut out).is_ok() {
                        claimed += 1;
                    }
                }
            }
            if claimed == 0 {
                let refusal = if choices > 0 { Refusal::NeedsChoice } else { Refusal::NothingToClaim };
                out.effects.push(Effect::Refused(refusal));
            }
        }
        QuestEvent::Refresh => {}
    }
    settle(book, progress, who.player, now, &mut roll, &mut out);
    start_over(book, progress, now, &mut out);
    out
}

/// An admin completes quest `quest` for a team, whatever its dependencies
/// and tasks: `who` gets its automatic rewards, and what it opens can
/// complete in turn. None when there is no such quest.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn force_complete(book: &Book, progress: &mut Progress, who: &Actor<'_>, quest: &str) -> Option<Outcome> {
    let n = book.find(quest)?;
    let mut out = Outcome::default();
    let mut roll = who.roll;
    if !progress.is_completed(quest) {
        for (t, task) in book.quest(n).tasks.iter().enumerate() {
            progress.tasks.insert(book.task_id(n, t), task.kind.goal());
        }
        complete(book, progress, who.player, who.now, n, &mut roll, &mut out);
    }
    settle(book, progress, who.player, who.now, &mut roll, &mut out);
    start_over(book, progress, who.now, &mut out);
    Some(out)
}

/// An admin resets quest `quest` for a team: its tasks, its completion, its
/// claims and how often it was repeated are forgotten. Quests completed
/// after it stay completed. None when there is no such quest; else whether
/// anything changed.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn reset_quest(book: &Book, progress: &mut Progress, quest: &str) -> Option<bool> {
    let n = book.find(quest)?;
    let mut changed = forget(book, progress, n);
    changed |= progress.repeats.remove(quest).is_some();
    Some(changed)
}

/// Forgets node `n`'s tasks, completion and claims. Whether there was
/// anything to forget.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
fn forget(book: &Book, progress: &mut Progress, n: usize) -> bool {
    let quest = &book.nodes()[n].id;
    let mut changed = progress.started.remove(quest).is_some();
    changed |= progress.completed.remove(quest).is_some();
    for t in 0..book.quest(n).tasks.len() {
        changed |= progress.tasks.remove(&book.task_id(n, t)).is_some();
    }
    for r in 0..book.quest(n).rewards.len() {
        changed |= progress.claimed.remove(&book.reward_id(n, r)).is_some();
    }
    changed
}

/// Whether a task of the pack asks for this advancement.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
fn names_advancement(book: &Book, advancement: &str) -> bool {
    (0..book.nodes().len()).any(|n| {
        book.quest(n)
            .tasks
            .iter()
            .any(|task| matches!(&task.kind, TaskKind::Advancement { advancement: wanted } if wanted == advancement))
    })
}

/// The task with this full id, if a request can still act on it.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn open_task(book: &Book, progress: &Progress, task: &str) -> Result<(usize, usize), Refusal> {
    let (n, t) = book.find_task(task).ok_or(Refusal::UnknownTask)?;
    if progress.is_completed(&book.nodes()[n].id) || progress.task(task) >= book.quest(n).tasks[t].kind.goal() {
        return Err(Refusal::AlreadyDone);
    }
    if !book.can_progress(n, progress) {
        return Err(Refusal::NotOpen);
    }
    if !book.task_counts(n, t, progress) {
        return Err(Refusal::NotYet);
    }
    Ok((n, t))
}

/// Moves task `t` of node `n` to `value`, at most its goal. A task that
/// reached its goal stays there.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn set_task(book: &Book, progress: &mut Progress, n: usize, t: usize, value: u64, now: u64, out: &mut Outcome) {
    let goal = book.quest(n).tasks[t].kind.goal();
    let id = book.task_id(n, t);
    let old = progress.task(&id);
    let value = value.min(goal);
    if old >= goal || value == old {
        return;
    }
    out.changed = true;
    if value == 0 {
        progress.tasks.remove(&id);
        return;
    }
    progress.tasks.insert(id.clone(), value);
    progress.started.entry(book.nodes()[n].id.clone()).or_insert(now);
    if value >= goal {
        out.effects.push(Effect::TaskCompleted { task: id });
    }
}

/// Records reward `r` of node `n` as claimed by `player`, and has it given.
/// A random reward draws its entry now; a choice reward gives entry
/// `choice`. Nothing is recorded when it is refused.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
#[allow(clippy::too_many_arguments, reason = "the engine's state, passed down")]
fn claim(
    book: &Book,
    progress: &mut Progress,
    player: &str,
    n: usize,
    r: usize,
    choice: Option<u32>,
    roll: &mut u64,
    out: &mut Outcome,
) -> Result<(), Refusal> {
    let reward = &book.quest(n).rewards[r];
    let (title, kind) = match &reward.kind {
        RewardKind::Random { table } => {
            let table = book.table(table).ok_or(Refusal::NoTable)?;
            let entry = draw(table, next_roll(roll)).ok_or(Refusal::NoTable)?;
            (entry.title.clone(), entry.kind.clone())
        }
        RewardKind::Choice { table } => {
            let table = book.table(table).ok_or(Refusal::NoTable)?;
            let choice = choice.ok_or(Refusal::NeedsChoice)?;
            let entry = table.entries.get(choice as usize).ok_or(Refusal::BadChoice)?;
            (entry.title.clone(), entry.kind.clone())
        }
        kind => (reward.title.clone(), kind.clone()),
    };
    let id = book.reward_id(n, r);
    progress.claimed.entry(id.clone()).or_default().insert(player.to_owned());
    out.changed = true;
    out.effects.push(Effect::Reward { reward: id, title, kind });
    Ok(())
}

/// Completes every quest that is open with all its tasks done. In dependency
/// order, so a quest that completes can complete those after it in one go.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn settle(book: &Book, progress: &mut Progress, player: &str, now: u64, roll: &mut u64, out: &mut Outcome) {
    for &n in book.order() {
        let node = &book.nodes()[n];
        if progress.is_completed(&node.id) {
            continue;
        }
        remembered(book, progress, n, now, out);
        if book.dependencies_met(n, progress) && book.tasks_done(n, progress) {
            complete(book, progress, player, now, n, roll, out);
        }
    }
}

/// Ticks node `n`'s advancement tasks whose advancement the team was seen
/// to complete, when they count.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
fn remembered(book: &Book, progress: &mut Progress, n: usize, now: u64, out: &mut Outcome) {
    for (t, task) in book.quest(n).tasks.iter().enumerate() {
        if let TaskKind::Advancement { advancement } = &task.kind
            && progress.advancements.contains(advancement)
            && book.task_counts(n, t, progress)
        {
            set_task(book, progress, n, t, 1, now, out);
        }
    }
}

/// Records node `n` as completed: its automatic rewards go to `player`, and
/// the chapter is announced if that was its last quest.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn complete(
    book: &Book,
    progress: &mut Progress,
    player: &str,
    now: u64,
    n: usize,
    roll: &mut u64,
    out: &mut Outcome,
) {
    let node = &book.nodes()[n];
    // A quest done again does not complete its chapter again.
    let first_time = !progress.ever_completed(&node.id);
    progress.completed.insert(node.id.clone(), now);
    progress.started.entry(node.id.clone()).or_insert(now);
    out.changed = true;
    out.effects.push(Effect::QuestCompleted { quest: node.id.clone() });

    let quest = book.quest(n);
    for (r, reward) in quest.rewards.iter().enumerate() {
        let auto = match reward.auto_claim {
            AutoClaim::Enabled => true,
            AutoClaim::Disabled => false,
            AutoClaim::Default => book.settings().auto_claim_rewards,
        };
        // A choice is the player's to make: never given by itself.
        if auto
            && !matches!(reward.kind, RewardKind::Choice { .. })
            && !progress.has_claimed(&book.reward_id(n, r), player)
        {
            let _ = claim(book, progress, player, n, r, None, roll, out);
        }
    }
    if first_time && !quest.optional && book.chapter_completed(node.chapter, progress) {
        out.effects.push(Effect::ChapterCompleted { chapter: book.chapter(node.chapter).id.clone() });
    }
}

/// Starts over every repeatable quest that is completed, has each of its
/// rewards claimed by someone and is past its cooldown. Once per event, and
/// after `settle`: a quest that starts over does not complete again at once.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
fn start_over(book: &Book, progress: &mut Progress, now: u64, out: &mut Outcome) {
    for n in 0..book.nodes().len() {
        let quest = book.quest(n);
        let id = &book.nodes()[n].id;
        if !quest.repeatable {
            continue;
        }
        let Some(&done_at) = progress.completed.get(id) else {
            continue;
        };
        let claimed = (0..quest.rewards.len())
            .all(|r| progress.claimed.get(&book.reward_id(n, r)).is_some_and(|players| !players.is_empty()));
        if !claimed || now < done_at.saturating_add(u64::from(quest.repeat_cooldown)) {
            continue;
        }
        forget(book, progress, n);
        let times = progress.repeats.entry(id.clone()).or_insert(0);
        *times = times.saturating_add(1);
        out.changed = true;
        out.effects.push(Effect::QuestStartedOver { quest: id.clone() });
    }
}

#[cfg(test)]
mod tests {
    use super::super::hq_validate::load;
    use super::*;

    const NOW: u64 = 1000;

    fn book(settings: &str, chapters: &[&str]) -> Book {
        let sources: Vec<(String, &str)> = chapters.iter().enumerate().map(|(i, text)| (format!("{i}.json"), *text)).collect();
        let sources: Vec<(&str, &str)> = sources.iter().map(|(name, text)| (name.as_str(), *text)).collect();
        let loaded = load(Some(settings), &sources);
        assert_eq!(loaded.problems, [], "the test pack has problems");
        Book::new(loaded.file)
    }

    /// Wood first; then a hunt (kills and a checkmark) and a hand-in; a bonus
    /// on the side.
    const CHAPTER: &str = r##"{ "id": "a", "title": "A", "quests": [
        { "id": "wood", "title": "t", "x": 0,
          "tasks": [ { "id": "logs", "type": "item", "item": "#minecraft:logs", "count": 8 } ],
          "rewards": [ { "id": "apple", "type": "item", "item": "apple", "count": 2 },
                       { "id": "xp", "type": "xp", "points": 20 } ] },
        { "id": "hunt", "title": "t", "x": 1, "dependencies": ["wood"],
          "tasks": [ { "id": "zombies", "type": "kill", "entity": "zombie", "count": 2 },
                     { "id": "ready", "type": "checkmark" } ],
          "rewards": [ { "id": "levels", "type": "xp_levels", "levels": 1, "auto_claim": "enabled" } ] },
        { "id": "give", "title": "t", "x": 2, "dependencies": ["wood"],
          "tasks": [ { "id": "planks", "type": "item", "item": "oak_planks", "count": 16, "consume": true } ] },
        { "id": "bonus", "title": "t", "x": 3, "optional": true,
          "tasks": [ { "id": "ok", "type": "checkmark" } ] }
    ] }"##;

    fn run(book: &Book, progress: &mut Progress, event: QuestEvent) -> Outcome {
        apply(book, progress, &Actor::new("steve", NOW), &event)
    }

    fn items(task: &str, count: u64) -> QuestEvent {
        QuestEvent::ItemCount { task: task.into(), count }
    }

    fn check(task: &str) -> QuestEvent {
        QuestEvent::Check { task: task.into() }
    }

    fn killed(entity: &str) -> QuestEvent {
        QuestEvent::Killed { entity: entity.into() }
    }

    fn refused(refusal: Refusal) -> Outcome {
        Outcome { changed: false, effects: vec![Effect::Refused(refusal)] }
    }

    fn task_completed(task: &str) -> Effect {
        Effect::TaskCompleted { task: task.into() }
    }

    fn quest_completed(quest: &str) -> Effect {
        Effect::QuestCompleted { quest: quest.into() }
    }

    /// Progress with the wood quest completed.
    fn after_wood(book: &Book) -> Progress {
        let mut progress = Progress::default();
        run(book, &mut progress, items("a/wood/logs", 8));
        assert!(progress.is_completed("a/wood"));
        progress
    }

    #[test]
    fn an_item_task_follows_the_inventory_until_done() {
        let book = book("{}", &[CHAPTER]);
        let mut progress = Progress::default();

        assert_eq!(run(&book, &mut progress, items("a/wood/logs", 0)), Outcome::default());
        let out = run(&book, &mut progress, items("a/wood/logs", 5));
        assert_eq!(out, Outcome { changed: true, effects: vec![] });
        assert_eq!((progress.task("a/wood/logs"), progress.started.get("a/wood")), (5, Some(&NOW)));
        // The same count again changes nothing.
        assert_eq!(run(&book, &mut progress, items("a/wood/logs", 5)), Outcome::default());
        // Logs put away: the task goes back down.
        assert!(run(&book, &mut progress, items("a/wood/logs", 3)).changed);
        assert_eq!(progress.task("a/wood/logs"), 3);
        assert!(run(&book, &mut progress, items("a/wood/logs", 0)).changed);
        assert!(!progress.tasks.contains_key("a/wood/logs"));

        let out = run(&book, &mut progress, items("a/wood/logs", 64));
        assert_eq!(out.effects, [task_completed("a/wood/logs"), quest_completed("a/wood")]);
        assert_eq!((progress.task("a/wood/logs"), progress.completed.get("a/wood")), (8, Some(&NOW)));
        // Done stays done.
        assert_eq!(run(&book, &mut progress, items("a/wood/logs", 0)), Outcome::default());
        assert_eq!(progress.task("a/wood/logs"), 8);
    }

    #[test]
    fn events_for_nothing_are_ignored() {
        let book = book("{}", &[CHAPTER]);
        let mut progress = Progress::default();
        for event in [
            items("a/none/logs", 8),
            items("nonsense", 8),
            // Not open yet (linear), and a hand-in task is not counted from the inventory.
            items("a/give/planks", 16),
            QuestEvent::Submitted { task: "a/give/planks".into(), count: 16 },
            // Not a hand-in task.
            QuestEvent::Submitted { task: "a/wood/logs".into(), count: 8 },
            killed("zombie"),
            killed("Not An Id"),
            QuestEvent::Refresh,
        ] {
            assert_eq!(run(&book, &mut progress, event.clone()), Outcome::default(), "{event:?}");
        }
        assert_eq!(progress, Progress::default());
    }

    #[test]
    fn hand_ins_add_up() {
        let book = book("{}", &[CHAPTER]);
        assert_eq!(submit_request(&book, &Progress::default(), "a/give/planks"), Err(Refusal::NotOpen));
        let mut progress = after_wood(&book);

        let planks = ItemMatch::Id("minecraft:oak_planks".into());
        let wanted = |missing| Wanted { task: "a/give/planks".into(), item: planks.clone(), missing, consume: true };
        assert_eq!(submit_request(&book, &progress, "a/give/planks"), Ok(wanted(16)));
        // Holding planks is not handing them in.
        assert_eq!(run(&book, &mut progress, items("a/give/planks", 64)), Outcome::default());

        let submitted = |count| QuestEvent::Submitted { task: "a/give/planks".into(), count };
        assert_eq!(run(&book, &mut progress, submitted(10)), Outcome { changed: true, effects: vec![] });
        assert_eq!(submit_request(&book, &progress, "a/give/planks"), Ok(wanted(6)));
        let out = run(&book, &mut progress, submitted(6));
        assert_eq!(out.effects, [task_completed("a/give/planks"), quest_completed("a/give")]);

        assert_eq!(submit_request(&book, &progress, "a/give/planks"), Err(Refusal::AlreadyDone));
        assert_eq!(submit_request(&book, &progress, "a/hunt/ready"), Err(Refusal::WrongTaskType));
        assert_eq!(submit_request(&book, &progress, "a/hunt/nope"), Err(Refusal::UnknownTask));
    }

    #[test]
    fn kills_count_for_open_quests() {
        let book = book("{}", &[CHAPTER]);
        let mut progress = after_wood(&book);
        assert_eq!(run(&book, &mut progress, killed("minecraft:skeleton")), Outcome::default());
        assert!(run(&book, &mut progress, killed("minecraft:zombie")).changed);
        // The short id is the same entity.
        let out = run(&book, &mut progress, killed("zombie"));
        assert_eq!(out.effects, [task_completed("a/hunt/zombies")]);
        assert_eq!(progress.task("a/hunt/zombies"), 2);
        assert_eq!(run(&book, &mut progress, killed("zombie")), Outcome::default());
        assert!(!progress.is_completed("a/hunt"));
    }

    #[test]
    fn a_kill_that_opens_a_quest_does_not_count_for_it() {
        let book = book(
            "{}",
            &[r#"{ "id": "a", "title": "A", "quests": [
                { "id": "first", "title": "t", "x": 0,
                  "tasks": [ { "id": "k", "type": "kill", "entity": "zombie", "count": 2 } ] },
                { "id": "then", "title": "t", "x": 1, "dependencies": ["first"], "dependency_mode": "one_started",
                  "tasks": [ { "id": "k", "type": "kill", "entity": "zombie", "count": 2 } ] } ] }"#],
        );
        let mut progress = Progress::default();
        run(&book, &mut progress, killed("zombie"));
        assert_eq!((progress.task("a/first/k"), progress.task("a/then/k")), (1, 0));
        run(&book, &mut progress, killed("zombie"));
        assert_eq!((progress.task("a/first/k"), progress.task("a/then/k")), (2, 1));
    }

    #[test]
    fn checkmarks_and_their_refusals() {
        let book = book("{}", &[CHAPTER]);
        let mut progress = Progress::default();
        assert_eq!(run(&book, &mut progress, check("a/hunt/ready")), refused(Refusal::NotOpen));
        assert_eq!(run(&book, &mut progress, check("a/hunt/nope")), refused(Refusal::UnknownTask));
        assert_eq!(run(&book, &mut progress, check("a/wood/logs")), refused(Refusal::WrongTaskType));

        let out = run(&book, &mut progress, check("a/bonus/ok"));
        assert_eq!(out.effects, [task_completed("a/bonus/ok"), quest_completed("a/bonus")]);
        assert_eq!(run(&book, &mut progress, check("a/bonus/ok")), refused(Refusal::AlreadyDone));

        let mut progress = after_wood(&book);
        assert_eq!(run(&book, &mut progress, check("a/hunt/ready")).effects, [task_completed("a/hunt/ready")]);
        assert_eq!(run(&book, &mut progress, check("a/hunt/ready")), refused(Refusal::AlreadyDone));
    }

    #[test]
    fn rewards_are_claimed_once_per_player() {
        let book = book("{}", &[CHAPTER]);
        let claim = |reward: &str| QuestEvent::Claim { reward: reward.into(), choice: None };
        let apples = Effect::Reward {
            reward: "a/wood/apple".into(),
            title: None,
            kind: RewardKind::Item { item: "minecraft:apple".into(), count: 2 },
        };
        let xp = Effect::Reward { reward: "a/wood/xp".into(), title: None, kind: RewardKind::Xp { points: 20 } };

        let mut progress = Progress::default();
        assert_eq!(run(&book, &mut progress, claim("a/wood/apple")), refused(Refusal::NotCompleted));
        assert_eq!(run(&book, &mut progress, claim("a/wood/pear")), refused(Refusal::UnknownReward));
        assert_eq!(run(&book, &mut progress, QuestEvent::ClaimAll), refused(Refusal::NothingToClaim));

        let mut progress = after_wood(&book);
        // Nothing was given on completion: auto-claim is off by default.
        assert!(progress.claimed.is_empty());
        let out = run(&book, &mut progress, claim("a/wood/apple"));
        assert_eq!(out, Outcome { changed: true, effects: vec![apples.clone()] });
        assert_eq!(run(&book, &mut progress, claim("a/wood/apple")), refused(Refusal::AlreadyClaimed));
        assert_eq!(run(&book, &mut progress, QuestEvent::ClaimAll).effects, [xp.clone()]);
        assert_eq!(run(&book, &mut progress, QuestEvent::ClaimAll), refused(Refusal::NothingToClaim));

        // Another player of the team gets their own.
        let out = apply(&book, &mut progress, &Actor::new("alex", NOW), &QuestEvent::ClaimAll);
        assert_eq!(out.effects, [apples, xp]);
        assert!(progress.has_claimed("a/wood/apple", "steve") && progress.has_claimed("a/wood/apple", "alex"));
    }

    #[test]
    fn auto_claim_gives_on_completion() {
        let levels = Effect::Reward { reward: "a/hunt/levels".into(), title: None, kind: RewardKind::XpLevels { levels: 1 } };

        // One reward asks for it.
        let book_off = book("{}", &[CHAPTER]);
        let mut progress = after_wood(&book_off);
        run(&book_off, &mut progress, killed("zombie"));
        run(&book_off, &mut progress, killed("zombie"));
        let out = run(&book_off, &mut progress, check("a/hunt/ready"));
        assert_eq!(out.effects, [task_completed("a/hunt/ready"), quest_completed("a/hunt"), levels]);
        assert!(progress.has_claimed("a/hunt/levels", "steve"));
        assert_eq!(run(&book_off, &mut progress, QuestEvent::Claim { reward: "a/hunt/levels".into(), choice: None }), refused(Refusal::AlreadyClaimed));

        // The pack asks for it, and a reward can say no.
        let chapter = r#"{ "id": "a", "title": "A", "quests": [ { "id": "q", "title": "t",
            "tasks": [ { "id": "ok", "type": "checkmark" } ],
            "rewards": [ { "id": "yes", "type": "xp", "points": 1 },
                         { "id": "no", "type": "xp", "points": 2, "auto_claim": "disabled" } ] } ] }"#;
        let book_on = book(r#"{ "auto_claim_rewards": true }"#, &[chapter]);
        let mut progress = Progress::default();
        let out = run(&book_on, &mut progress, check("a/q/ok"));
        assert_eq!(
            out.effects,
            [
                task_completed("a/q/ok"),
                quest_completed("a/q"),
                Effect::Reward { reward: "a/q/yes".into(), title: None, kind: RewardKind::Xp { points: 1 } },
                Effect::ChapterCompleted { chapter: "a".into() },
            ]
        );
        assert!(!progress.has_claimed("a/q/no", "steve"));
    }

    #[test]
    fn flexible_quests_progress_early_and_complete_in_a_chain() {
        // `kills` is flexible work done ahead; `gate` and `end` have no task.
        let chapter = r#"{ "id": "a", "title": "A", "progression": "flexible", "quests": [
            { "id": "start", "title": "t", "x": 0, "tasks": [ { "id": "ok", "type": "checkmark" } ] },
            { "id": "kills", "title": "t", "x": 1, "dependencies": ["start"],
              "tasks": [ { "id": "k", "type": "kill", "entity": "pig" } ] },
            { "id": "gate", "title": "t", "x": 2, "dependencies": ["kills"] },
            { "id": "end", "title": "t", "x": 3, "dependencies": ["gate"] } ] }"#;
        let book = book("{}", &[chapter]);
        let mut progress = Progress::default();
        let out = run(&book, &mut progress, killed("pig"));
        assert_eq!(out.effects, [task_completed("a/kills/k")]);
        assert!(!progress.is_completed("a/kills"));

        let out = run(&book, &mut progress, check("a/start/ok"));
        assert_eq!(
            out.effects,
            [
                task_completed("a/start/ok"),
                quest_completed("a/start"),
                quest_completed("a/kills"),
                quest_completed("a/gate"),
                quest_completed("a/end"),
                Effect::ChapterCompleted { chapter: "a".into() },
            ]
        );
    }

    #[test]
    fn a_quest_without_tasks_completes_on_refresh() {
        let chapter = r#"{ "id": "a", "title": "A", "quests": [
            { "id": "welcome", "title": "t", "rewards": [ { "id": "r", "type": "xp", "points": 5 } ] } ] }"#;
        let book = book("{}", &[chapter]);
        let mut progress = Progress::default();
        let out = run(&book, &mut progress, QuestEvent::Refresh);
        assert_eq!(out.effects, [quest_completed("a/welcome"), Effect::ChapterCompleted { chapter: "a".into() }]);
        assert_eq!((progress.started.get("a/welcome"), progress.completed.get("a/welcome")), (Some(&NOW), Some(&NOW)));
        assert_eq!(run(&book, &mut progress, QuestEvent::Refresh), Outcome::default());
    }

    #[test]
    fn a_chapter_completes_once_and_optional_quests_do_not_count() {
        let book = book("{}", &[CHAPTER]);
        let mut progress = after_wood(&book);
        run(&book, &mut progress, killed("zombie"));
        run(&book, &mut progress, killed("zombie"));
        let out = run(&book, &mut progress, check("a/hunt/ready"));
        assert!(!out.effects.contains(&Effect::ChapterCompleted { chapter: "a".into() }));

        let out = run(&book, &mut progress, QuestEvent::Submitted { task: "a/give/planks".into(), count: 16 });
        assert_eq!(out.effects.last(), Some(&Effect::ChapterCompleted { chapter: "a".into() }));
        // The optional quest, later, does not announce the chapter again.
        let out = run(&book, &mut progress, check("a/bonus/ok"));
        assert_eq!(out.effects, [task_completed("a/bonus/ok"), quest_completed("a/bonus")]);
    }

    #[test]
    fn wanted_items_lists_open_unfinished_item_tasks() {
        let book = book("{}", &[CHAPTER]);
        let logs = ItemMatch::Tag("minecraft:logs".into());
        let mut progress = Progress::default();
        assert_eq!(
            wanted_items(&book, &progress),
            [Wanted { task: "a/wood/logs".into(), item: logs.clone(), missing: 8, consume: false }]
        );
        run(&book, &mut progress, items("a/wood/logs", 3));
        assert_eq!(wanted_items(&book, &progress)[0].missing, 5);

        run(&book, &mut progress, items("a/wood/logs", 8));
        let wanted = wanted_items(&book, &progress);
        assert_eq!(wanted.len(), 1);
        assert_eq!((wanted[0].task.as_str(), wanted[0].missing, wanted[0].consume), ("a/give/planks", 16, true));
    }

    #[test]
    fn an_admin_can_complete_and_reset_a_quest() {
        let book = book("{}", &[CHAPTER]);
        let mut progress = Progress::default();
        assert_eq!(force_complete(&book, &mut progress, &Actor::new("steve", NOW), "a/none"), None);
        assert_eq!(reset_quest(&book, &mut progress, "a/none"), None);
        assert_eq!(reset_quest(&book, &mut progress, "a/hunt"), Some(false));

        // Hunt is locked behind wood: forced, it completes anyway, with its automatic reward.
        let out = force_complete(&book, &mut progress, &Actor::new("steve", NOW), "a/hunt").unwrap();
        let levels = Effect::Reward { reward: "a/hunt/levels".into(), title: None, kind: RewardKind::XpLevels { levels: 1 } };
        assert_eq!(out, Outcome { changed: true, effects: vec![quest_completed("a/hunt"), levels] });
        assert_eq!((progress.task("a/hunt/zombies"), progress.task("a/hunt/ready")), (2, 1));
        assert!(!progress.is_completed("a/wood"));
        // Again: nothing more.
        assert_eq!(force_complete(&book, &mut progress, &Actor::new("steve", NOW), "a/hunt"), Some(Outcome::default()));

        assert_eq!(reset_quest(&book, &mut progress, "a/hunt"), Some(true));
        assert_eq!(progress, Progress::default());

        // Forcing a quest lets what waited on it complete: a flexible quest done ahead.
        let chapter = r#"{ "id": "f", "title": "F", "progression": "flexible", "quests": [
            { "id": "gate", "title": "t", "x": 0, "tasks": [ { "id": "ok", "type": "checkmark" } ] },
            { "id": "after", "title": "t", "x": 1, "dependencies": ["gate"],
              "tasks": [ { "id": "k", "type": "kill", "entity": "pig" } ] } ] }"#;
        let book = self::book("{}", &[chapter]);
        let mut progress = Progress::default();
        run(&book, &mut progress, killed("pig"));
        let out = force_complete(&book, &mut progress, &Actor::new("steve", NOW), "f/gate").unwrap();
        assert_eq!(
            out.effects,
            [quest_completed("f/gate"), quest_completed("f/after"), Effect::ChapterCompleted { chapter: "f".into() }]
        );
    }

    #[test]
    fn refusals_have_a_text() {
        assert_eq!(Refusal::NothingToClaim.text(), "There is nothing to claim.");
    }
}
