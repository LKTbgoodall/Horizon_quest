// Reading a quest pack (quests.json and chapters/*.json) and checking it.
// A chapter with an error is left out, not the whole pack; an error in
// quests.json refuses the pack. Every problem is reported with where it is,
// so a pack author can fix the file. Pure: no game types (it reads files).

use super::hq_model::{
    Chapter, FORMAT_VERSION, Quest, QuestFile, RewardKind, Settings, TaskKind, dependency_id, quest_id, resource_id,
    valid_id,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

const SETTINGS_FILE: &str = "quests.json";
const ID_RULE: &str = "an `id` made of lowercase letters, digits, `_`, `-` or `.`";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Worth fixing; the chapter still loads.
    Warning,
    /// The chapter (or, in quests.json, the pack) is left out.
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub severity: Severity,
    /// The file, and the quest in it: `chapters/01_start.json > first_wood`.
    pub place: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(f, "{kind}: {}: {}", self.place, self.message)
    }
}

/// A pack as it loaded: the chapters that are sound, in book order, and
/// everything found wrong on the way.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Loaded {
    pub file: QuestFile,
    pub problems: Vec<Problem>,
}

impl Loaded {
    pub fn errors(&self) -> usize {
        self.problems.iter().filter(|p| p.severity == Severity::Error).count()
    }

    fn refused(place: &str, message: String) -> Self {
        Self { file: QuestFile::default(), problems: vec![error(place, message)] }
    }
}

fn error(place: &str, message: String) -> Problem {
    Problem { severity: Severity::Error, place: place.to_owned(), message }
}

/// A problem and the chapter it is in (None: quests.json).
struct Found {
    chapter: Option<usize>,
    problem: Problem,
}

#[derive(Default)]
struct Finder {
    found: Vec<Found>,
}

impl Finder {
    fn add(&mut self, severity: Severity, chapter: Option<usize>, place: &str, message: impl Into<String>) {
        let problem = Problem { severity, place: place.to_owned(), message: message.into() };
        self.found.push(Found { chapter, problem });
    }

    fn error(&mut self, chapter: Option<usize>, place: &str, message: impl Into<String>) {
        self.add(Severity::Error, chapter, place, message);
    }

    fn warning(&mut self, chapter: Option<usize>, place: &str, message: impl Into<String>) {
        self.add(Severity::Warning, chapter, place, message);
    }
}

/// The pack in folder `dir`. A missing quests.json means default settings, a
/// missing `chapters` folder an empty book.
pub fn load_dir(dir: &Path) -> Loaded {
    let settings = match std::fs::read_to_string(dir.join(SETTINGS_FILE)) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Loaded::refused(SETTINGS_FILE, format!("cannot be read: {e}")),
    };
    let mut problems = Vec::new();
    let mut sources = Vec::new();
    match std::fs::read_dir(dir.join("chapters")) {
        Ok(entries) => {
            let mut paths: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect();
            paths.sort();
            for path in paths {
                let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                match std::fs::read_to_string(&path) {
                    Ok(text) => sources.push((name, text)),
                    Err(e) => problems.push(error(&format!("chapters/{name}"), format!("cannot be read: {e}"))),
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => problems.push(error("chapters", format!("cannot be read: {e}"))),
    }
    let sources: Vec<(&str, &str)> = sources.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
    let mut loaded = load(settings.as_deref(), &sources);
    problems.append(&mut loaded.problems);
    loaded.problems = problems;
    loaded
}

/// The pack made of quests.json's text (None: default settings) and the
/// chapter files' (name, text), in file-name order.
pub fn load(settings: Option<&str>, chapters: &[(&str, &str)]) -> Loaded {
    let settings = match settings.map(|text| serde_json::from_str::<Settings>(without_bom(text))) {
        None => Settings::default(),
        Some(Ok(settings)) => settings,
        Some(Err(e)) => return Loaded::refused(SETTINGS_FILE, format!("not valid settings: {e}")),
    };
    if settings.version > FORMAT_VERSION {
        let message = format!(
            "this pack is version {}, newer than this mod reads ({FORMAT_VERSION}): update Horizon Quests",
            settings.version
        );
        return Loaded::refused(SETTINGS_FILE, message);
    }

    let mut problems = Vec::new();
    let mut entries: Vec<(String, Chapter)> = Vec::new();
    for (name, text) in chapters {
        let place = format!("chapters/{name}");
        match serde_json::from_str::<Chapter>(without_bom(text)) {
            Ok(chapter) => entries.push((place, chapter)),
            Err(e) => problems.push(error(&place, format!("not a valid chapter: {e}"))),
        }
    }
    // Stable: chapters of the same `order` stay in file-name order.
    entries.sort_by_key(|(_, chapter)| chapter.order);

    // Leaving a chapter out can break another's dependencies, so look again
    // until what is left is sound.
    loop {
        let found = find(&settings, &entries);
        if found.iter().any(|f| f.chapter.is_none() && f.problem.severity == Severity::Error) {
            problems.extend(found.into_iter().map(|f| f.problem));
            entries.clear();
            break;
        }
        let bad: BTreeSet<usize> =
            found.iter().filter(|f| f.problem.severity == Severity::Error).filter_map(|f| f.chapter).collect();
        if bad.is_empty() {
            problems.extend(found.into_iter().map(|f| f.problem));
            break;
        }
        problems.extend(found.into_iter().filter(|f| f.chapter.is_some_and(|c| bad.contains(&c))).map(|f| f.problem));
        let mut index = 0;
        entries.retain(|_| {
            index += 1;
            !bad.contains(&(index - 1))
        });
    }

    let chapters = entries.into_iter().map(|(_, chapter)| chapter).collect();
    Loaded { file: QuestFile { settings, chapters }, problems }
}

/// Notepad on Windows starts files with a byte-order mark, which is not JSON.
fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn good_size(size: f32) -> bool {
    size.is_finite() && size > 0.0
}

/// Everything wrong with the pack. `chapters` are (place, chapter).
fn find(settings: &Settings, chapters: &[(String, Chapter)]) -> Vec<Found> {
    let mut f = Finder::default();

    if !good_size(settings.default_size) {
        f.error(None, SETTINGS_FILE, "`default_size` must be above 0");
    }
    let mut groups = BTreeSet::new();
    for group in &settings.groups {
        if !valid_id(&group.id) {
            f.warning(None, SETTINGS_FILE, format!("group `{}` needs {ID_RULE}", group.id));
        } else if !groups.insert(group.id.as_str()) {
            f.warning(None, SETTINGS_FILE, format!("two groups have the id `{}`", group.id));
        }
    }

    // Every well-named quest by full id, and where it is.
    let mut quests: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut chapter_ids = BTreeSet::new();
    for (c, (place, chapter)) in chapters.iter().enumerate() {
        let named = if !valid_id(&chapter.id) {
            f.error(Some(c), place, format!("the chapter needs {ID_RULE}"));
            false
        } else if !chapter_ids.insert(chapter.id.as_str()) {
            f.error(Some(c), place, format!("another chapter already has the id `{}`", chapter.id));
            false
        } else {
            true
        };
        if chapter.title.trim().is_empty() {
            f.warning(Some(c), place, "the chapter has no `title`");
        }
        if let Some(group) = chapter.group.as_deref().filter(|group| !groups.contains(group)) {
            f.warning(Some(c), place, format!("group `{group}` is not in {SETTINGS_FILE}"));
        }
        check_look(&mut f, c, place, chapter.icon.as_deref(), chapter.default_size, "default_size");

        let mut quest_ids = BTreeSet::new();
        let mut cells = BTreeSet::new();
        for (q, quest) in chapter.quests.iter().enumerate() {
            let place = quest_place(place, q, quest);
            if !valid_id(&quest.id) {
                f.error(Some(c), &place, format!("the quest needs {ID_RULE}"));
            } else if !quest_ids.insert(quest.id.as_str()) {
                f.error(Some(c), &place, "another quest of this chapter already has this id");
            } else if named {
                quests.insert(quest_id(&chapter.id, &quest.id), (c, q));
            }
            // Adding 0 makes -0 the same cell as 0.
            if !cells.insert(((quest.x + 0.0).to_bits(), (quest.y + 0.0).to_bits())) {
                f.warning(Some(c), &place, format!("another quest is already at x {}, y {}", quest.x, quest.y));
            }
            check_quest(&mut f, c, &place, quest);
        }
    }

    // Dependencies, now that every quest is known.
    let index: BTreeMap<&str, usize> = quests.keys().enumerate().map(|(i, id)| (id.as_str(), i)).collect();
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); quests.len()];
    for (c, (place, chapter)) in chapters.iter().enumerate() {
        for (q, quest) in chapter.quests.iter().enumerate() {
            let place = quest_place(place, q, quest);
            let own = quest_id(&chapter.id, &quest.id);
            let mut seen = BTreeSet::new();
            for dependency in &quest.dependencies {
                let full = dependency_id(&chapter.id, dependency);
                if !seen.insert(full.clone()) {
                    f.warning(Some(c), &place, format!("dependency `{dependency}` is listed twice"));
                } else if full == own {
                    f.error(Some(c), &place, "the quest depends on itself");
                } else if let Some(&to) = index.get(full.as_str()) {
                    if quests.get(&own) == Some(&(c, q)) {
                        edges[index[own.as_str()]].push(to);
                    }
                } else {
                    f.error(Some(c), &place, format!("unknown dependency `{dependency}`"));
                }
            }
            match quest.min_required {
                Some(0) => f.warning(Some(c), &place, "`min_required` 0 does nothing"),
                Some(min) if min as usize > seen.len() => f.error(
                    Some(c),
                    &place,
                    format!("`min_required` is {min} but the quest has {} dependencies", seen.len()),
                ),
                _ => {}
            }
        }
    }
    let caught = caught_in_cycles(&edges);
    for (i, &(c, q)) in quests.values().enumerate() {
        if caught[i] {
            let (place, chapter) = &chapters[c];
            f.error(Some(c), &quest_place(place, q, &chapter.quests[q]), "the quest is caught in a dependency cycle");
        }
    }
    f.found
}

/// `chapters/01_start.json > first_wood`, or `… > quest #2` without a usable
/// id.
fn quest_place(chapter: &str, q: usize, quest: &Quest) -> String {
    if valid_id(&quest.id) { format!("{chapter} > {}", quest.id) } else { format!("{chapter} > quest #{}", q + 1) }
}

fn check_look(f: &mut Finder, c: usize, place: &str, icon: Option<&str>, size: Option<f32>, size_name: &str) {
    if let Some(icon) = icon.filter(|icon| resource_id(icon).is_none()) {
        f.error(Some(c), place, format!("icon `{icon}` is not an item id"));
    }
    if size.is_some_and(|size| !good_size(size)) {
        f.error(Some(c), place, format!("`{size_name}` must be above 0"));
    }
}

/// What is wrong inside one quest, its dependencies aside.
fn check_quest(f: &mut Finder, c: usize, place: &str, quest: &Quest) {
    if quest.title.trim().is_empty() {
        f.warning(Some(c), place, "the quest has no `title`");
    }
    if !quest.x.is_finite() || !quest.y.is_finite() {
        f.error(Some(c), place, "`x` and `y` must be plain numbers");
    }
    check_look(f, c, place, quest.icon.as_deref(), quest.size, "size");

    let mut ids = BTreeSet::new();
    for (t, task) in quest.tasks.iter().enumerate() {
        if !valid_id(&task.id) {
            f.error(Some(c), place, format!("task #{} needs {ID_RULE}", t + 1));
        } else if !ids.insert(task.id.as_str()) {
            f.error(Some(c), place, format!("two tasks have the id `{}`", task.id));
        }
        if task.kind.goal() == 0 {
            f.error(Some(c), place, format!("task `{}`: `count` must be at least 1", task.id));
        }
        if let TaskKind::Kill { entity, .. } = &task.kind
            && resource_id(entity).is_none()
        {
            f.error(Some(c), place, format!("task `{}`: `{entity}` is not an entity id", task.id));
        }
    }

    let mut ids = BTreeSet::new();
    for (r, reward) in quest.rewards.iter().enumerate() {
        if !valid_id(&reward.id) {
            f.error(Some(c), place, format!("reward #{} needs {ID_RULE}", r + 1));
        } else if !ids.insert(reward.id.as_str()) {
            f.error(Some(c), place, format!("two rewards have the id `{}`", reward.id));
        }
        match &reward.kind {
            RewardKind::Item { item, count } => {
                if resource_id(item).is_none() {
                    f.error(Some(c), place, format!("reward `{}`: `{item}` is not an item id", reward.id));
                }
                if *count == 0 {
                    f.error(Some(c), place, format!("reward `{}`: `count` must be at least 1", reward.id));
                }
            }
            RewardKind::Xp { points: 0 } | RewardKind::XpLevels { levels: 0 } => {
                f.warning(Some(c), place, format!("reward `{}` gives nothing", reward.id));
            }
            RewardKind::Xp { .. } | RewardKind::XpLevels { .. } => {}
        }
    }
}

/// Which quests can never open because of a dependency cycle: those on one,
/// and those between two. `edges[i]` lists what quest `i` depends on.
fn caught_in_cycles(edges: &[Vec<usize>]) -> Vec<bool> {
    let mut alive = vec![true; edges.len()];
    // Take away, until nothing moves, the quests that depend on nothing left
    // and those nothing left depends on.
    loop {
        let mut needed = vec![false; edges.len()];
        for (i, dependencies) in edges.iter().enumerate() {
            if alive[i] {
                for &d in dependencies {
                    needed[d] = true;
                }
            }
        }
        let mut moved = false;
        for i in 0..edges.len() {
            if alive[i] && (!needed[i] || !edges[i].iter().any(|&d| alive[d])) {
                alive[i] = false;
                moved = true;
            }
        }
        if !moved {
            return alive;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chapter file with these quests: (id, dependencies, extra fields).
    fn chapter(id: &str, quests: &[(&str, &[&str], &str)]) -> String {
        let quests: Vec<String> = quests
            .iter()
            .enumerate()
            .map(|(i, (quest, dependencies, extra))| {
                format!(
                    r#"{{ "id": "{quest}", "title": "{quest}", "x": {i}, "dependencies": {dependencies:?}{extra} }}"#
                )
            })
            .collect();
        format!(r#"{{ "id": "{id}", "title": "{id}", "quests": [{}] }}"#, quests.join(","))
    }

    fn ids(loaded: &Loaded) -> Vec<&str> {
        loaded.file.chapters.iter().map(|c| c.id.as_str()).collect()
    }

    fn messages(loaded: &Loaded) -> Vec<String> {
        loaded.problems.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn a_sound_pack_loads_clean() {
        let a = chapter("a", &[("one", &[], ""), ("two", &["one"], ""), ("three", &["one", "two"], "")]);
        let b = chapter("b", &[("far", &["a/three"], r#", "min_required": 1"#)]);
        let loaded = load(Some("{}"), &[("a.json", &a), ("b.json", &b)]);
        assert_eq!(messages(&loaded), Vec::<String>::new());
        assert_eq!(ids(&loaded), ["a", "b"]);
        assert_eq!(loaded.errors(), 0);
    }

    #[test]
    fn chapters_sort_by_order_then_file_name() {
        let chapters = [
            ("1.json", r#"{ "id": "late", "title": "t", "order": 5 }"#),
            ("2.json", r#"{ "id": "first", "title": "t", "order": -1 }"#),
            ("3.json", r#"{ "id": "mid_a", "title": "t" }"#),
            ("4.json", r#"{ "id": "mid_b", "title": "t" }"#),
        ];
        assert_eq!(ids(&load(None, &chapters)), ["first", "mid_a", "mid_b", "late"]);
    }

    #[test]
    fn a_broken_chapter_is_left_out_alone() {
        let good = chapter("good", &[("one", &[], "")]);
        let loaded = load(None, &[("bad.json", "{ not json"), ("good.json", &good)]);
        assert_eq!(ids(&loaded), ["good"]);
        assert_eq!(loaded.errors(), 1);
        assert!(messages(&loaded)[0].starts_with("error: chapters/bad.json: not a valid chapter:"));
    }

    #[test]
    fn broken_settings_refuse_the_pack() {
        let good = chapter("good", &[("one", &[], "")]);
        for settings in ["{ nope", r#"{ "version": 999 }"#, r#"{ "default_size": 0 }"#] {
            let loaded = load(Some(settings), &[("good.json", &good)]);
            assert!(loaded.file.chapters.is_empty(), "{settings}");
            assert_eq!(loaded.errors(), 1, "{settings}");
            assert_eq!(loaded.problems[0].place, "quests.json");
        }
    }

    #[test]
    fn a_byte_order_mark_is_fine() {
        let good = format!("\u{feff}{}", chapter("good", &[("one", &[], "")]));
        let loaded = load(Some("\u{feff}{}"), &[("good.json", &good)]);
        assert_eq!((ids(&loaded), loaded.problems.len()), (vec!["good"], 0));
    }

    #[test]
    fn bad_and_duplicate_ids_are_errors() {
        let twice = chapter("a", &[("one", &[], ""), ("one", &[], "")]);
        let loaded = load(None, &[("a.json", &twice)]);
        assert_eq!(
            messages(&loaded),
            ["error: chapters/a.json > one: another quest of this chapter already has this id"]
        );

        let unnamed = r#"{ "id": "a", "title": "t", "quests": [ { "title": "t" }, { "id": "Bad Id", "title": "t", "x": 1 } ] }"#;
        let loaded = load(None, &[("a.json", unnamed)]);
        assert_eq!(loaded.errors(), 2);
        assert!(messages(&loaded)[0].starts_with("error: chapters/a.json > quest #1: the quest needs an `id`"));
        assert!(messages(&loaded)[1].starts_with("error: chapters/a.json > quest #2:"));

        let first = chapter("same", &[("one", &[], "")]);
        let loaded = load(None, &[("1.json", &first), ("2.json", &first), ("3.json", r#"{ "title": "t" }"#)]);
        assert_eq!(ids(&loaded), ["same"]);
        assert_eq!(
            messages(&loaded),
            [
                "error: chapters/2.json: another chapter already has the id `same`",
                "error: chapters/3.json: the chapter needs an `id` made of lowercase letters, digits, `_`, `-` or `.`",
            ]
        );
    }

    #[test]
    fn tasks_and_rewards_are_checked() {
        let quest = r#", "icon": "Not An Item", "size": -1,
            "tasks": [ { "id": "t", "type": "checkmark" }, { "id": "t", "type": "kill", "entity": "Big Zombie", "count": 0 },
                       { "type": "checkmark" } ],
            "rewards": [ { "id": "r", "type": "item", "item": "No Item", "count": 0 }, { "id": "r", "type": "xp", "points": 0 },
                         { "type": "xp_levels", "levels": 1 } ]"#;
        let loaded = load(None, &[("a.json", &chapter("a", &[("q", &[], quest)]))]);
        assert!(loaded.file.chapters.is_empty());
        let expected = [
            "error: chapters/a.json > q: icon `Not An Item` is not an item id",
            "error: chapters/a.json > q: `size` must be above 0",
            "error: chapters/a.json > q: two tasks have the id `t`",
            "error: chapters/a.json > q: task `t`: `count` must be at least 1",
            "error: chapters/a.json > q: task `t`: `Big Zombie` is not an entity id",
            "error: chapters/a.json > q: task #3 needs an `id` made of lowercase letters, digits, `_`, `-` or `.`",
            "error: chapters/a.json > q: reward `r`: `No Item` is not an item id",
            "error: chapters/a.json > q: reward `r`: `count` must be at least 1",
            "error: chapters/a.json > q: two rewards have the id `r`",
            "warning: chapters/a.json > q: reward `r` gives nothing",
            "error: chapters/a.json > q: reward #3 needs an `id` made of lowercase letters, digits, `_`, `-` or `.`",
        ];
        assert_eq!(messages(&loaded), expected);
    }

    #[test]
    fn dependencies_are_checked() {
        let a = chapter(
            "a",
            &[
                ("one", &["one"], ""),
                ("two", &["nowhere", "b/missing"], ""),
                ("three", &["two", "two"], r#", "min_required": 2"#),
            ],
        );
        let loaded = load(None, &[("a.json", &a)]);
        assert_eq!(
            messages(&loaded),
            [
                "error: chapters/a.json > one: the quest depends on itself",
                "error: chapters/a.json > two: unknown dependency `nowhere`",
                "error: chapters/a.json > two: unknown dependency `b/missing`",
                "warning: chapters/a.json > three: dependency `two` is listed twice",
                "error: chapters/a.json > three: `min_required` is 2 but the quest has 1 dependencies",
            ]
        );
    }

    #[test]
    fn cycles_are_found_across_chapters() {
        // a/one → b/two → a/three → a/one is a cycle; a/free and b/after are not on it.
        let a = chapter("a", &[("one", &["three"], ""), ("three", &["b/two"], ""), ("free", &[], "")]);
        let b = chapter("b", &[("two", &["a/one"], ""), ("after", &["two"], "")]);
        let c = chapter("c", &[("alone", &[], "")]);
        let loaded = load(None, &[("a.json", &a), ("b.json", &b), ("c.json", &c)]);
        assert_eq!(ids(&loaded), ["c"]);
        assert_eq!(
            messages(&loaded),
            [
                "error: chapters/a.json > one: the quest is caught in a dependency cycle",
                "error: chapters/a.json > three: the quest is caught in a dependency cycle",
                "error: chapters/b.json > two: the quest is caught in a dependency cycle",
            ]
        );
    }

    #[test]
    fn leaving_a_chapter_out_breaks_those_that_need_it() {
        let bad = chapter("bad", &[("one", &["nowhere"], "")]);
        let needs = chapter("needs", &[("two", &["bad/one"], "")]);
        let fine = chapter("fine", &[("three", &[], "")]);
        let loaded = load(None, &[("1.json", &bad), ("2.json", &needs), ("3.json", &fine)]);
        assert_eq!(ids(&loaded), ["fine"]);
        assert_eq!(
            messages(&loaded),
            [
                "error: chapters/1.json > one: unknown dependency `nowhere`",
                "error: chapters/2.json > two: unknown dependency `bad/one`",
            ]
        );
    }

    #[test]
    fn warnings_keep_the_chapter() {
        let chapters = [(
            "a.json",
            r#"{ "id": "a", "group": "ghost", "quests": [
                 { "id": "one", "min_required": 0 }, { "id": "two", "x": -0.0 } ] }"#,
        )];
        let settings = r#"{ "groups": [ { "id": "main" }, { "id": "main" }, { "id": "Bad" } ] }"#;
        let loaded = load(Some(settings), &chapters);
        assert_eq!((ids(&loaded), loaded.errors()), (vec!["a"], 0));
        assert_eq!(
            messages(&loaded),
            [
                "warning: quests.json: two groups have the id `main`",
                "warning: quests.json: group `Bad` needs an `id` made of lowercase letters, digits, `_`, `-` or `.`",
                "warning: chapters/a.json: the chapter has no `title`",
                "warning: chapters/a.json: group `ghost` is not in quests.json",
                "warning: chapters/a.json > one: the quest has no `title`",
                "warning: chapters/a.json > two: another quest is already at x -0, y 0",
                "warning: chapters/a.json > two: the quest has no `title`",
                "warning: chapters/a.json > one: `min_required` 0 does nothing",
            ]
        );
    }

    #[test]
    fn a_folder_loads() {
        let dir = std::env::temp_dir().join(format!("hq-validate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load_dir(&dir), Loaded::default());

        std::fs::create_dir_all(dir.join("chapters")).unwrap();
        std::fs::write(dir.join("quests.json"), r#"{ "progression": "flexible" }"#).unwrap();
        std::fs::write(dir.join("chapters/02_b.json"), chapter("b", &[("two", &["a/one"], "")])).unwrap();
        std::fs::write(dir.join("chapters/01_a.json"), chapter("a", &[("one", &[], "")])).unwrap();
        std::fs::write(dir.join("chapters/03_c.json"), "oops").unwrap();
        std::fs::write(dir.join("chapters/notes.txt"), "not a chapter").unwrap();
        let loaded = load_dir(&dir);
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(ids(&loaded), ["a", "b"]);
        assert_eq!(loaded.file.settings.progression, super::super::hq_model::Progression::Flexible);
        assert_eq!(loaded.errors(), 1);
        assert_eq!(loaded.problems[0].place, "chapters/03_c.json");
    }
}
