// The quest book's rules that only read: a `Book` is a loaded pack with its
// dependencies resolved, and it answers which quests are open, which show,
// and what state each is in for a team's progress. Both halves use it: the
// server to decide, the client's book to draw. Pure: no game types.
use super::hq_model::{
    Chapter, DependencyMode, Progress, Progression, Quest, QuestFile, Settings, Shape, Visibility, child_id,
    dependency_id, quest_id,
};
use std::collections::BTreeMap;
use std::ops::Range;

/// A quest in the book, with what its chapter and the pack default for it.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Node {
    /// The full id, `chapter/quest`.
    pub id: String,
    /// Its chapter, and its place in that chapter's `quests`.
    pub chapter: usize,
    pub quest: usize,
    /// The nodes it depends on, and those that depend on it.
    pub dependencies: Vec<usize>,
    pub dependents: Vec<usize>,
    pub progression: Progression,
    pub shape: Shape,
    pub size: f32,
}

/// What the book shows a quest as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum QuestState {
    /// Its dependencies are not met.
    Locked,
    /// Open, nothing done yet.
    Available,
    InProgress,
    /// Completed, with a reward this player has not claimed.
    RewardsWaiting,
    /// Completed, nothing left to claim.
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Status {
    pub visible: bool,
    pub state: QuestState,
}

/// A pack ready to use. It expects one that hq_validate loaded: a dependency
/// that names no quest is ignored here, not reported.
#[derive(Clone, Debug)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Book {
    file: QuestFile,
    /// Every quest, chapter after chapter.
    nodes: Vec<Node>,
    by_id: BTreeMap<String, usize>,
    /// Each chapter's nodes.
    chapters: Vec<Range<usize>>,
    /// The nodes, dependencies before dependents.
    order: Vec<usize>,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Book {
    pub fn new(file: QuestFile) -> Self {
        let settings = &file.settings;
        let mut nodes = Vec::new();
        let mut by_id = BTreeMap::new();
        let mut chapters = Vec::new();
        for (c, chapter) in file.chapters.iter().enumerate() {
            let start = nodes.len();
            for (q, quest) in chapter.quests.iter().enumerate() {
                let id = quest_id(&chapter.id, &quest.id);
                by_id.entry(id.clone()).or_insert(nodes.len());
                nodes.push(Node {
                    id,
                    chapter: c,
                    quest: q,
                    dependencies: Vec::new(),
                    dependents: Vec::new(),
                    progression: chapter.progression.unwrap_or(settings.progression),
                    shape: quest.shape.or(chapter.default_shape).unwrap_or(settings.default_shape),
                    size: quest.size.or(chapter.default_size).unwrap_or(settings.default_size),
                });
            }
            chapters.push(start..nodes.len());
        }

        for n in 0..nodes.len() {
            let chapter = &file.chapters[nodes[n].chapter];
            let mut dependencies: Vec<usize> = chapter.quests[nodes[n].quest]
                .dependencies
                .iter()
                .filter_map(|dependency| by_id.get(&dependency_id(&chapter.id, dependency)).copied())
                .filter(|&d| d != n)
                .collect();
            dependencies.sort_unstable();
            dependencies.dedup();
            for &d in &dependencies {
                nodes[d].dependents.push(n);
            }
            nodes[n].dependencies = dependencies;
        }

        // Dependencies first. Quests on a cycle (a checked pack has none)
        // never get their turn: they go last.
        let mut waiting: Vec<usize> = nodes.iter().map(|node| node.dependencies.len()).collect();
        let mut order: Vec<usize> = (0..nodes.len()).filter(|&n| waiting[n] == 0).collect();
        let mut next = 0;
        while next < order.len() {
            for &d in &nodes[order[next]].dependents {
                waiting[d] -= 1;
                if waiting[d] == 0 {
                    order.push(d);
                }
            }
            next += 1;
        }
        order.extend((0..nodes.len()).filter(|&n| waiting[n] > 0));

        Self { file, nodes, by_id, chapters, order }
    }

    pub fn file(&self) -> &QuestFile {
        &self.file
    }

    pub fn settings(&self) -> &Settings {
        &self.file.settings
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Every node, dependencies before dependents.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    pub fn chapter(&self, c: usize) -> &Chapter {
        &self.file.chapters[c]
    }

    /// The nodes of chapter `c`.
    pub fn chapter_nodes(&self, c: usize) -> Range<usize> {
        self.chapters[c].clone()
    }

    pub fn quest(&self, n: usize) -> &Quest {
        let node = &self.nodes[n];
        &self.file.chapters[node.chapter].quests[node.quest]
    }

    /// The node of the quest with this full id.
    pub fn find(&self, quest: &str) -> Option<usize> {
        self.by_id.get(quest).copied()
    }

    /// The node and task index of the task with this full id.
    pub fn find_task(&self, task: &str) -> Option<(usize, usize)> {
        let (quest, task) = task.rsplit_once('/')?;
        let n = self.find(quest)?;
        Some((n, self.quest(n).tasks.iter().position(|t| t.id == task)?))
    }

    /// The node and reward index of the reward with this full id.
    pub fn find_reward(&self, reward: &str) -> Option<(usize, usize)> {
        let (quest, reward) = reward.rsplit_once('/')?;
        let n = self.find(quest)?;
        Some((n, self.quest(n).rewards.iter().position(|r| r.id == reward)?))
    }

    /// The full id of task `t` of node `n`.
    pub fn task_id(&self, n: usize, t: usize) -> String {
        child_id(&self.nodes[n].id, &self.quest(n).tasks[t].id)
    }

    /// The full id of reward `r` of node `n`.
    pub fn reward_id(&self, n: usize, r: usize) -> String {
        child_id(&self.nodes[n].id, &self.quest(n).rewards[r].id)
    }

    /// Whether the quest is open: its dependencies are what its
    /// `dependency_mode` (or `min_required`) asks.
    pub fn dependencies_met(&self, n: usize, progress: &Progress) -> bool {
        let dependencies = &self.nodes[n].dependencies;
        if dependencies.is_empty() {
            return true;
        }
        let completed = |d: &usize| progress.is_completed(&self.nodes[*d].id);
        let started = |d: &usize| progress.is_started(&self.nodes[*d].id);
        let quest = self.quest(n);
        if let Some(min) = quest.min_required.filter(|&min| min > 0) {
            return dependencies.iter().filter(|d| completed(d)).count() >= min as usize;
        }
        match quest.dependency_mode {
            DependencyMode::AllCompleted => dependencies.iter().all(completed),
            DependencyMode::OneCompleted => dependencies.iter().any(completed),
            DependencyMode::AllStarted => dependencies.iter().all(started),
            DependencyMode::OneStarted => dependencies.iter().any(started),
        }
    }

    /// Whether the quest's tasks count right now: it is not completed, and
    /// it is open or its progression is flexible.
    pub fn can_progress(&self, n: usize, progress: &Progress) -> bool {
        let node = &self.nodes[n];
        !progress.is_completed(&node.id)
            && (node.progression == Progression::Flexible || self.dependencies_met(n, progress))
    }

    /// Whether every task of the quest reached its goal.
    pub fn tasks_done(&self, n: usize, progress: &Progress) -> bool {
        let quest = self.quest(n);
        quest.tasks.iter().all(|task| progress.task(&child_id(&self.nodes[n].id, &task.id)) >= task.kind.goal())
    }

    /// Every node's status for this progress, as `player` sees it (rewards
    /// are claimed per player).
    pub fn statuses(&self, progress: &Progress, player: &str) -> Vec<Status> {
        let mut statuses = vec![Status { visible: false, state: QuestState::Locked }; self.nodes.len()];
        // In order: a quest's visibility can hang on its dependencies'.
        for &n in &self.order {
            let node = &self.nodes[n];
            let quest = self.quest(n);
            let completed = progress.is_completed(&node.id);
            let open = self.dependencies_met(n, progress);
            let state = if completed {
                let waiting =
                    quest.rewards.iter().any(|reward| !progress.has_claimed(&child_id(&node.id, &reward.id), player));
                if waiting { QuestState::RewardsWaiting } else { QuestState::Done }
            } else if !open {
                QuestState::Locked
            } else if progress.is_started(&node.id) {
                QuestState::InProgress
            } else {
                QuestState::Available
            };
            let shown = completed
                || match quest.visibility {
                    Visibility::Always => true,
                    Visibility::UntilDepsVisible => node.dependencies.iter().all(|&d| statuses[d].visible),
                    Visibility::UntilDepsComplete => open,
                    Visibility::Secret => false,
                };
            statuses[n] = Status { visible: shown && !self.chapter(node.chapter).hidden, state };
        }
        statuses
    }

    /// How many of chapter `c`'s quests are completed, out of how many;
    /// optional quests are not counted.
    pub fn chapter_tally(&self, c: usize, progress: &Progress) -> (usize, usize) {
        let counted = self.chapter_nodes(c).filter(|&n| !self.quest(n).optional);
        let (mut completed, mut total) = (0, 0);
        for n in counted {
            total += 1;
            completed += usize::from(progress.is_completed(&self.nodes[n].id));
        }
        (completed, total)
    }

    /// Whether every quest of chapter `c` that counts is completed.
    pub fn chapter_completed(&self, c: usize, progress: &Progress) -> bool {
        let (completed, total) = self.chapter_tally(c, progress);
        total > 0 && completed == total
    }
}

#[cfg(test)]
mod tests {
    use super::super::hq_validate::load;
    use super::*;

    /// A book from chapter files, which must be sound.
    fn book(settings: &str, chapters: &[&str]) -> Book {
        let sources: Vec<(String, &str)> = chapters.iter().enumerate().map(|(i, text)| (format!("{i}.json"), *text)).collect();
        let sources: Vec<(&str, &str)> = sources.iter().map(|(name, text)| (name.as_str(), *text)).collect();
        let loaded = load(Some(settings), &sources);
        assert_eq!(loaded.problems, [], "the test pack has problems");
        Book::new(loaded.file)
    }

    /// Progress with these quests completed and those started.
    fn progress(completed: &[&str], started: &[&str]) -> Progress {
        let mut progress = Progress::default();
        for quest in completed {
            progress.completed.insert((*quest).to_owned(), 1);
        }
        for quest in started {
            progress.started.insert((*quest).to_owned(), 1);
        }
        progress
    }

    const TREE: &str = r#"{ "id": "a", "title": "A", "quests": [
        { "id": "root", "title": "t", "x": 0 },
        { "id": "left", "title": "t", "x": 1, "dependencies": ["root"] },
        { "id": "right", "title": "t", "x": 2, "dependencies": ["root"] },
        { "id": "all", "title": "t", "x": 3, "dependencies": ["left", "right"] },
        { "id": "one", "title": "t", "x": 4, "dependencies": ["left", "right"], "dependency_mode": "one_completed" },
        { "id": "all_started", "title": "t", "x": 5, "dependencies": ["left", "right"], "dependency_mode": "all_started" },
        { "id": "one_started", "title": "t", "x": 6, "dependencies": ["left", "right"], "dependency_mode": "one_started" },
        { "id": "two_of", "title": "t", "x": 7, "dependencies": ["root", "left", "right"], "min_required": 2,
          "dependency_mode": "one_started" }
    ] }"#;

    #[test]
    fn a_book_resolves_defaults_and_dependencies() {
        let a = r#"{ "id": "a", "title": "A", "default_shape": "circle", "progression": "flexible", "quests": [
            { "id": "late", "title": "t", "x": 0, "dependencies": ["early", "b/far"], "shape": "diamond", "size": 2,
              "tasks": [ { "id": "t1", "type": "checkmark" } ],
              "rewards": [ { "id": "r1", "type": "xp", "points": 1 } ] },
            { "id": "early", "title": "t", "x": 1 } ] }"#;
        let b = r#"{ "id": "b", "title": "B", "default_size": 0.5, "quests": [ { "id": "far", "title": "t" } ] }"#;
        let book = book(r#"{ "default_shape": "hexagon", "default_size": 1.5 }"#, &[a, b]);

        let ids: Vec<&str> = book.nodes().iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["a/late", "a/early", "b/far"]);
        assert_eq!((book.find("a/early"), book.find("b/far"), book.find("a/none")), (Some(1), Some(2), None));
        assert_eq!((book.chapter_nodes(0), book.chapter_nodes(1)), (0..2, 2..3));
        assert_eq!(book.quest(2).id, "far");

        let late = &book.nodes()[0];
        assert_eq!((late.dependencies.as_slice(), late.shape, late.size), (&[1, 2][..], Shape::Diamond, 2.0));
        assert_eq!(late.progression, Progression::Flexible);
        let early = &book.nodes()[1];
        assert_eq!((early.dependents.as_slice(), early.shape, early.size), (&[0][..], Shape::Circle, 1.5));
        let far = &book.nodes()[2];
        assert_eq!((far.shape, far.size, far.progression), (Shape::Hexagon, 0.5, Progression::Linear));

        // Dependencies come before dependents.
        assert_eq!(book.order(), [1, 2, 0]);

        assert_eq!(book.find_task("a/late/t1"), Some((0, 0)));
        assert_eq!(book.find_reward("a/late/r1"), Some((0, 0)));
        assert_eq!((book.task_id(0, 0), book.reward_id(0, 0)), ("a/late/t1".to_owned(), "a/late/r1".to_owned()));
        for missing in ["a/late/t2", "a/none/t1", "t1", ""] {
            assert_eq!((book.find_task(missing), book.find_reward(missing)), (None, None), "{missing}");
        }
    }

    #[test]
    fn dependency_modes() {
        let book = book("{}", &[TREE]);
        let met = |progress: &Progress| -> Vec<&str> {
            (0..book.nodes().len())
                .filter(|&n| book.dependencies_met(n, progress))
                .map(|n| book.quest(n).id.as_str())
                .collect()
        };
        assert_eq!(met(&progress(&[], &[])), ["root"]);
        assert_eq!(met(&progress(&["a/root"], &[])), ["root", "left", "right"]);
        // Left started: enough for one_started only.
        assert_eq!(met(&progress(&["a/root"], &["a/left"])), ["root", "left", "right", "one_started"]);
        // Left completed: completed counts as started, and makes 2 of 3.
        assert_eq!(
            met(&progress(&["a/root", "a/left"], &[])),
            ["root", "left", "right", "one", "one_started", "two_of"]
        );
        assert_eq!(
            met(&progress(&["a/root", "a/left"], &["a/right"])),
            ["root", "left", "right", "one", "all_started", "one_started", "two_of"]
        );
        assert_eq!(met(&progress(&["a/root", "a/left", "a/right"], &[])).len(), 8);
        // min_required counts completed ones, whatever dependency_mode says.
        assert_eq!(met(&progress(&[], &["a/root", "a/left", "a/right"])), ["root", "all_started", "one_started"]);
    }

    #[test]
    fn linear_quests_wait_for_their_dependencies_and_flexible_ones_do_not() {
        let chapter = |progression: &str| {
            format!(
                r#"{{ "id": "a", "title": "A", "progression": "{progression}", "quests": [
                    {{ "id": "root", "title": "t", "x": 0 }},
                    {{ "id": "next", "title": "t", "x": 1, "dependencies": ["root"] }} ] }}"#
            )
        };
        let linear = book("{}", &[&chapter("linear")]);
        assert!(linear.can_progress(0, &progress(&[], &[])));
        assert!(!linear.can_progress(1, &progress(&[], &[])));
        assert!(linear.can_progress(1, &progress(&["a/root"], &[])));
        // A completed quest is over.
        assert!(!linear.can_progress(0, &progress(&["a/root"], &[])));

        let flexible = book("{}", &[&chapter("flexible")]);
        assert!(flexible.can_progress(1, &progress(&[], &[])));
        assert!(!flexible.can_progress(1, &progress(&["a/next"], &[])));
    }

    #[test]
    fn tasks_done_needs_every_goal() {
        let book = book(
            "{}",
            &[r#"{ "id": "a", "title": "A", "quests": [
                { "id": "q", "title": "t", "x": 0, "tasks": [
                    { "id": "logs", "type": "item", "item": "oak_log", "count": 8 },
                    { "id": "ok", "type": "checkmark" } ] },
                { "id": "empty", "title": "t", "x": 1 } ] }"#],
        );
        let mut progress = Progress::default();
        assert!(!book.tasks_done(0, &progress));
        // No task: nothing left to do.
        assert!(book.tasks_done(1, &progress));
        progress.tasks.insert("a/q/logs".into(), 8);
        assert!(!book.tasks_done(0, &progress));
        progress.tasks.insert("a/q/ok".into(), 1);
        assert!(book.tasks_done(0, &progress));
        progress.tasks.insert("a/q/logs".into(), 7);
        assert!(!book.tasks_done(0, &progress));
    }

    #[test]
    fn states() {
        let book = book(
            "{}",
            &[r#"{ "id": "a", "title": "A", "quests": [
                { "id": "root", "title": "t", "x": 0, "rewards": [ { "id": "r1", "type": "xp", "points": 1 },
                                                                 { "id": "r2", "type": "xp", "points": 1 } ] },
                { "id": "next", "title": "t", "x": 1, "dependencies": ["root"] },
                { "id": "last", "title": "t", "x": 2, "dependencies": ["next"] } ] }"#],
        );
        let states = |progress: &Progress, player: &str| -> Vec<QuestState> {
            book.statuses(progress, player).iter().map(|s| s.state).collect()
        };
        use QuestState::*;
        assert_eq!(states(&progress(&[], &[]), "steve"), [Available, Locked, Locked]);
        assert_eq!(states(&progress(&[], &["a/root"]), "steve"), [InProgress, Locked, Locked]);

        let mut done = progress(&["a/root"], &["a/next"]);
        assert_eq!(states(&done, "steve"), [RewardsWaiting, InProgress, Locked]);
        done.claimed.entry("a/root/r1".into()).or_default().insert("steve".into());
        assert_eq!(states(&done, "steve"), [RewardsWaiting, InProgress, Locked]);
        done.claimed.entry("a/root/r2".into()).or_default().insert("steve".into());
        assert_eq!(states(&done, "steve"), [Done, InProgress, Locked]);
        // Claims are per player.
        assert_eq!(states(&done, "alex"), [RewardsWaiting, InProgress, Locked]);

        // No reward: nothing to wait for.
        assert_eq!(states(&progress(&["a/root", "a/next"], &[]), "steve")[1..], [Done, Available]);
    }

    #[test]
    fn visibility() {
        let shown = r#"{ "id": "a", "title": "A", "quests": [
            { "id": "root", "title": "t", "x": 0 },
            { "id": "secret", "title": "t", "x": 1, "visibility": "secret" },
            { "id": "after_root", "title": "t", "x": 2, "dependencies": ["root"], "visibility": "until_deps_complete" },
            { "id": "after_secret", "title": "t", "x": 3, "dependencies": ["secret"], "visibility": "until_deps_visible" },
            { "id": "chained", "title": "t", "x": 4, "dependencies": ["after_secret", "root"],
              "visibility": "until_deps_visible" } ] }"#;
        let hidden = r#"{ "id": "h", "title": "H", "hidden": true, "quests": [ { "id": "ghost", "title": "t" } ] }"#;
        let book = book("{}", &[shown, hidden]);
        let visible = |progress: &Progress| -> Vec<&str> {
            let statuses = book.statuses(progress, "steve");
            (0..statuses.len()).filter(|&n| statuses[n].visible).map(|n| book.quest(n).id.as_str()).collect()
        };
        assert_eq!(visible(&progress(&[], &[])), ["root"]);
        assert_eq!(visible(&progress(&["a/root"], &[])), ["root", "after_root"]);
        // A secret shows once completed, and what waited on it follows.
        assert_eq!(
            visible(&progress(&["a/root", "a/secret"], &[])),
            ["root", "secret", "after_root", "after_secret", "chained"]
        );
        // Completed quests show whatever their rule; a hidden chapter's never do.
        assert_eq!(visible(&progress(&["a/chained", "h/ghost"], &[])), ["root", "chained"]);
    }

    #[test]
    fn chapter_tallies_skip_optional_quests() {
        let book = book(
            "{}",
            &[
                r#"{ "id": "a", "title": "A", "quests": [
                    { "id": "one", "title": "t", "x": 0 }, { "id": "two", "title": "t", "x": 1 },
                    { "id": "extra", "title": "t", "x": 2, "optional": true } ] }"#,
                r#"{ "id": "empty", "title": "E" }"#,
            ],
        );
        assert_eq!(book.chapter_tally(0, &progress(&[], &[])), (0, 2));
        assert_eq!(book.chapter_tally(0, &progress(&["a/one", "a/extra"], &[])), (1, 2));
        assert!(!book.chapter_completed(0, &progress(&["a/one", "a/extra"], &[])));
        assert!(book.chapter_completed(0, &progress(&["a/one", "a/two"], &[])));
        assert_eq!(book.chapter_tally(1, &progress(&[], &[])), (0, 0));
        assert!(!book.chapter_completed(1, &progress(&[], &[])));
    }

    #[test]
    fn an_unchecked_pack_with_a_cycle_does_not_hang() {
        let file: QuestFile = serde_json::from_str(
            r#"{ "chapters": [ { "id": "a", "quests": [
                { "id": "one", "dependencies": ["two", "one", "nowhere"] },
                { "id": "two", "dependencies": ["one"], "visibility": "until_deps_visible" },
                { "id": "free" } ] } ] }"#,
        )
        .unwrap();
        let book = Book::new(file);
        assert_eq!(book.order(), [2, 0, 1]);
        assert_eq!(book.nodes()[0].dependencies, [1]);
        let statuses = book.statuses(&Progress::default(), "steve");
        assert_eq!(statuses.iter().map(|s| s.state).collect::<Vec<_>>(), [
            QuestState::Locked,
            QuestState::Locked,
            QuestState::Available
        ]);
    }
}
