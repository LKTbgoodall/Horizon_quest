// Reading a quest pack (quests.json and chapters/*.json) and checking it.
// A chapter with an error is left out, not the whole pack; an error in
// quests.json refuses the pack. Every problem is reported with where it is,
// so a pack author can fix the file. Pure: no game types (it reads files).
use super::hq_model::{
    AutoClaim, Chapter, FORMAT_VERSION, Quest, QuestFile, RewardKind, RewardTable, Settings, TaskKind, dependency_id,
    quest_id, resource_id, valid_id,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

const SETTINGS_FILE: &str = "quests.json";
const TABLES_FOLDER: &str = "reward_tables";
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
    let chapters = read_folder(dir, "chapters", &mut problems);
    let tables = read_folder(dir, TABLES_FOLDER, &mut problems);
    fn as_refs(files: &[(String, String)]) -> Vec<(&str, &str)> {
        files.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect()
    }
    let mut loaded = load_pack(settings.as_deref(), &as_refs(&chapters), &as_refs(&tables));
    problems.append(&mut loaded.problems);
    loaded.problems = problems;
    loaded
}

/// The folder under `data/<namespace>/` a data pack keeps its quests in:
/// `quests.json` (the settings), `chapters/*.json`, `reward_tables/*.json`.
pub const DATA_FOLDER: &str = "horizon_quests";

/// The pack the data packs hold, from the files Horizon read in their
/// `data/<namespace>/horizon_quests/` folders: (`<namespace>:<path>`, JSON),
/// sorted by id. None when they hold no chapter. Chapters and tables of every
/// namespace make one pack; the settings are the first namespace's
/// `quests.json`.
pub fn load_data(files: &[(String, serde_json::Value)]) -> Option<Loaded> {
    let mut problems = Vec::new();
    let mut settings: Option<String> = None;
    let mut chapters = Vec::new();
    let mut tables = Vec::new();
    for (id, json) in files {
        let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
        let place = format!("data/{namespace}/{DATA_FOLDER}/{path}.json");
        let text = json.to_string();
        let name = format!("{namespace}:{path}.json");
        if path == "quests" {
            if settings.is_none() {
                settings = Some(text);
            } else {
                problems.push(Problem {
                    severity: Severity::Warning,
                    place,
                    message: "another data pack's quests.json came first: this one is not used".to_owned(),
                });
            }
        } else if let Some(file) = path.strip_prefix("chapters/") {
            chapters.push((format!("{namespace}:{file}.json"), text));
        } else if let Some(file) = path.strip_prefix(&format!("{TABLES_FOLDER}/")) {
            tables.push((format!("{namespace}:{file}.json"), text));
        } else {
            let message = format!("not a quests.json, a chapters/ file or a {TABLES_FOLDER}/ file: left out ({name})");
            problems.push(Problem { severity: Severity::Warning, place, message });
        }
    }
    if chapters.is_empty() {
        return None;
    }
    fn as_refs(files: &[(String, String)]) -> Vec<(&str, &str)> {
        files.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect()
    }
    let mut loaded = load_pack(settings.as_deref(), &as_refs(&chapters), &as_refs(&tables));
    problems.append(&mut loaded.problems);
    loaded.problems = problems;
    Some(loaded)
}

/// The `.json` files of `dir/folder`, as (name, text) in file-name order. A
/// missing folder has none; what cannot be read is a problem.
fn read_folder(dir: &Path, folder: &str, problems: &mut Vec<Problem>) -> Vec<(String, String)> {
    let mut sources = Vec::new();
    match std::fs::read_dir(dir.join(folder)) {
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
                    Err(e) => problems.push(error(&format!("{folder}/{name}"), format!("cannot be read: {e}"))),
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => problems.push(error(folder, format!("cannot be read: {e}"))),
    }
    sources
}

/// The pack made of quests.json's text (None: default settings) and the
/// chapter files' (name, text), in file-name order: one without reward
/// tables.
#[cfg(test)]
pub fn load(settings: Option<&str>, chapters: &[(&str, &str)]) -> Loaded {
    load_pack(settings, chapters, &[])
}

/// The pack made of quests.json's text (None: default settings), the
/// chapter files' and the reward table files' (name, text), each in
/// file-name order.
pub fn load_pack(settings: Option<&str>, chapters: &[(&str, &str)], tables: &[(&str, &str)]) -> Loaded {
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
    // Reward tables first: the chapters' rewards name them. A table with an
    // error is left out, and then the chapters that needed it.
    let mut reward_tables: Vec<RewardTable> = Vec::new();
    for (name, text) in tables {
        let place = format!("{TABLES_FOLDER}/{name}");
        match serde_json::from_str::<RewardTable>(without_bom(text)) {
            Ok(mut table) => {
                let found = check_table(&place, &table, &reward_tables);
                let sound = !found.iter().any(|problem| problem.severity == Severity::Error);
                problems.extend(found);
                if sound {
                    for entry in &mut table.entries {
                        write_reward_ids_in_full(&mut entry.kind);
                    }
                    reward_tables.push(table);
                }
            }
            Err(e) => problems.push(error(&place, format!("not a valid reward table: {e}"))),
        }
    }
    let table_ids: BTreeSet<&str> = reward_tables.iter().map(|table| table.id.as_str()).collect();

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
        let found = find(&settings, &entries, &table_ids);
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

    let mut chapters: Vec<Chapter> = entries.into_iter().map(|(_, chapter)| chapter).collect();
    chapters.iter_mut().for_each(write_ids_in_full);
    Loaded { file: QuestFile { settings, chapters, reward_tables }, problems }
}

/// `apple` becomes `minecraft:apple`.
fn full(id: &mut String) {
    if let Some(whole) = resource_id(id) {
        *id = whole;
    }
}

/// Writes the chapter's item, entity, dimension and advancement ids in full
/// (`apple` becomes `minecraft:apple`), and a location's corners low then
/// high, so the rest of the mod compares them as they are.
fn write_ids_in_full(chapter: &mut Chapter) {
    chapter.icon.iter_mut().for_each(full);
    for quest in &mut chapter.quests {
        quest.icon.iter_mut().for_each(full);
        for task in &mut quest.tasks {
            match &mut task.kind {
                TaskKind::Kill { entity, .. } => full(entity),
                TaskKind::Dimension { dimension } => full(dimension),
                TaskKind::Advancement { advancement } => full(advancement),
                TaskKind::Location { dimension, from, to } => {
                    dimension.iter_mut().for_each(full);
                    for axis in 0..3 {
                        if from[axis] > to[axis] {
                            std::mem::swap(&mut from[axis], &mut to[axis]);
                        }
                    }
                }
                TaskKind::Item { .. } | TaskKind::Craft { .. } | TaskKind::Checkmark | TaskKind::XpLevels { .. } => {}
            }
        }
        for reward in &mut quest.rewards {
            write_reward_ids_in_full(&mut reward.kind);
        }
    }
}

fn write_reward_ids_in_full(kind: &mut RewardKind) {
    if let RewardKind::Item { item, .. } = kind {
        full(item);
    }
}

/// What is wrong with what a reward (of a quest or of a table) gives, each
/// as its severity and a message to put after the reward's name.
fn kind_problems(kind: &RewardKind) -> Vec<(Severity, String)> {
    let mut found = Vec::new();
    match kind {
        RewardKind::Item { item, count, max_count } => {
            if resource_id(item).is_none() {
                found.push((Severity::Error, format!("`{item}` is not an item id")));
            }
            if *count == 0 {
                found.push((Severity::Error, "`count` must be at least 1".to_owned()));
            }
            if max_count.is_some_and(|max| max < *count) {
                found.push((Severity::Error, "`max_count` must be at least `count`".to_owned()));
            }
        }
        RewardKind::Xp { points: 0 } | RewardKind::XpLevels { levels: 0 } => {
            found.push((Severity::Warning, "gives nothing".to_owned()));
        }
        RewardKind::Xp { .. } | RewardKind::XpLevels { .. } => {}
        RewardKind::Command { command } => {
            if command.trim().is_empty() {
                found.push((Severity::Error, "the `command` is empty".to_owned()));
            } else if command.contains(['\n', '\r']) {
                found.push((Severity::Error, "the `command` must be one line".to_owned()));
            }
        }
        RewardKind::Toast { .. } | RewardKind::Random { .. } | RewardKind::Choice { .. } => {}
    }
    found
}

/// What is wrong with reward table `table`, read from `place`, given the
/// tables read before it.
fn check_table(place: &str, table: &RewardTable, earlier: &[RewardTable]) -> Vec<Problem> {
    let mut found = Vec::new();
    let mut add = |severity: Severity, message: String| {
        found.push(Problem { severity, place: place.to_owned(), message });
    };
    if !valid_id(&table.id) {
        add(Severity::Error, format!("the reward table needs {ID_RULE}"));
    } else if earlier.iter().any(|other| other.id == table.id) {
        add(Severity::Error, format!("another reward table already has the id `{}`", table.id));
    }
    if table.entries.is_empty() {
        add(Severity::Error, "the reward table has no `entries`".to_owned());
    } else if table.entries.iter().all(|entry| entry.weight == 0) {
        add(Severity::Error, "every entry has `weight` 0: nothing can be drawn".to_owned());
    }
    for (e, entry) in table.entries.iter().enumerate() {
        if entry.kind.table().is_some() {
            add(Severity::Error, format!("entry #{}: an entry cannot be another table", e + 1));
        }
        for (severity, message) in kind_problems(&entry.kind) {
            add(severity, format!("entry #{}: {message}", e + 1));
        }
    }
    found
}

/// Notepad on Windows starts files with a byte-order mark, which is not JSON.
fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn good_size(size: f32) -> bool {
    size.is_finite() && size > 0.0
}

/// Everything wrong with the pack. `chapters` are (place, chapter);
/// `tables` are the ids of the reward tables that loaded.
fn find(settings: &Settings, chapters: &[(String, Chapter)], tables: &BTreeSet<&str>) -> Vec<Found> {
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
            check_quest(&mut f, c, &place, quest, tables);
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
fn check_quest(f: &mut Finder, c: usize, place: &str, quest: &Quest, tables: &BTreeSet<&str>) {
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
            let field = if matches!(task.kind, TaskKind::XpLevels { .. }) { "levels" } else { "count" };
            f.error(Some(c), place, format!("task `{}`: `{field}` must be at least 1", task.id));
        }
        let (id, what) = match &task.kind {
            TaskKind::Kill { entity, .. } => (Some(entity), "an entity id"),
            TaskKind::Dimension { dimension } => (Some(dimension), "a dimension id"),
            TaskKind::Location { dimension, .. } => (dimension.as_ref(), "a dimension id"),
            TaskKind::Advancement { advancement } => (Some(advancement), "an advancement id"),
            TaskKind::Item { .. } | TaskKind::Craft { .. } | TaskKind::Checkmark | TaskKind::XpLevels { .. } => {
                (None, "")
            }
        };
        if let Some(id) = id.filter(|id| resource_id(id).is_none()) {
            f.error(Some(c), place, format!("task `{}`: `{id}` is not {what}", task.id));
        }
    }
    // A quest that starts over must cost something each time: tasks that
    // only look at what the player has or is would complete it again at
    // once, for ever.
    let used_up = |kind: &TaskKind| {
        matches!(
            kind,
            TaskKind::Kill { .. } | TaskKind::Craft { .. } | TaskKind::Checkmark | TaskKind::Item { consume: true, .. }
        )
    };
    if quest.repeatable && !quest.tasks.iter().any(|task| used_up(&task.kind)) {
        f.error(
            Some(c),
            place,
            "a repeatable quest needs a task that is used up each time: a kill, a craft, a checkmark or a hand-in",
        );
    }
    if !quest.repeatable && quest.repeat_cooldown > 0 {
        f.warning(Some(c), place, "`repeat_cooldown` does nothing without `repeatable`");
    }

    let mut ids = BTreeSet::new();
    for (r, reward) in quest.rewards.iter().enumerate() {
        if !valid_id(&reward.id) {
            f.error(Some(c), place, format!("reward #{} needs {ID_RULE}", r + 1));
        } else if !ids.insert(reward.id.as_str()) {
            f.error(Some(c), place, format!("two rewards have the id `{}`", reward.id));
        }
        for (severity, message) in kind_problems(&reward.kind) {
            f.add(severity, Some(c), place, format!("reward `{}`: {message}", reward.id));
        }
        if let Some(table) = reward.kind.table().filter(|table| !tables.contains(table)) {
            f.error(Some(c), place, format!("reward `{}`: unknown reward table `{table}`", reward.id));
        }
        if matches!(reward.kind, RewardKind::Choice { .. }) && reward.auto_claim == AutoClaim::Enabled {
            f.warning(
                Some(c),
                place,
                format!("reward `{}`: a choice is never claimed automatically", reward.id),
            );
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
            "warning: chapters/a.json > q: reward `r`: gives nothing",
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
    fn data_packs_make_one_pack_of_every_namespace() {
        use serde_json::json;
        let file = |id: &str, value: serde_json::Value| (id.to_owned(), value);
        let files = [
            file("alpha:chapters/one", json!({ "id": "one", "title": "One", "quests": [
                { "id": "q", "title": "Q", "rewards": [ { "id": "r", "type": "random", "table": "loot" } ] } ] })),
            file("alpha:notes", json!({ "hello": 1 })),
            file("alpha:quests", json!({ "detection_delay": 40 })),
            file("alpha:reward_tables/loot", json!({ "id": "loot", "entries": [ { "type": "xp", "points": 3 } ] })),
            file("beta:chapters/two", json!({ "id": "two", "title": "Two", "quests": [ { "id": "q", "title": "Q" } ] })),
            file("beta:quests", json!({ "detection_delay": 99 })),
        ];
        let loaded = load_data(&files).unwrap();
        let ids: Vec<&str> = loaded.file.chapters.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["one", "two"]);
        assert_eq!(loaded.file.reward_tables.len(), 1);
        // The first namespace's settings; the other's and the stray file are warned about.
        assert_eq!(loaded.file.settings.detection_delay, 40);
        assert_eq!(loaded.errors(), 0, "{:?}", loaded.problems);
        let places: Vec<&str> = loaded.problems.iter().map(|p| p.place.as_str()).collect();
        assert_eq!(places, ["data/alpha/horizon_quests/notes.json", "data/beta/horizon_quests/quests.json"]);

        // No chapter: no pack from the data packs.
        assert!(load_data(&[file("alpha:quests", json!({}))]).is_none());
        assert!(load_data(&[]).is_none());
        // A broken chapter is reported under its namespace.
        let broken = load_data(&[file("gamma:chapters/bad", json!({ "id": "Bad Id" }))]).unwrap();
        assert!(broken.errors() > 0 && broken.problems.iter().any(|p| p.place.contains("gamma:bad.json")), "{:?}", broken.problems);
    }

    #[test]
    fn reward_tables_are_checked_and_a_bad_one_takes_its_chapters_out() {
        let quest = |table: &str| {
            format!(
                r#", "tasks": [ {{ "id": "t", "type": "checkmark" }} ],
                    "rewards": [ {{ "id": "r", "type": "random", "table": "{table}" }} ]"#
            )
        };
        let (good, empty, nowhere) = (quest("good"), quest("empty"), quest("nowhere"));
        let chapters = [
            ("a.json", chapter("a", &[("q", &[], &good)])),
            ("b.json", chapter("b", &[("q", &[], &empty)])),
            ("c.json", chapter("c", &[("q", &[], &nowhere)])),
        ];
        let chapters: Vec<(&str, &str)> = chapters.iter().map(|(name, text)| (*name, text.as_str())).collect();
        let tables = [
            (
                "1.json",
                r#"{ "id": "good", "entries": [ { "type": "item", "item": "apple" }, { "weight": 0, "type": "xp", "points": 0 } ] }"#,
            ),
            ("2.json", r#"{ "id": "empty", "entries": [] }"#),
            ("3.json", r#"{ "id": "good", "entries": [ { "type": "xp", "points": 1 } ] }"#),
            (
                "4.json",
                r#"{ "id": "nested", "entries": [ { "type": "random", "table": "good" }, { "type": "item", "item": "No Item" } ] }"#,
            ),
            ("5.json", r#"{ "id": "idle", "entries": [ { "weight": 0, "type": "xp", "points": 1 } ] }"#),
            ("6.json", "not json"),
        ];
        let loaded = load_pack(None, &chapters, &tables);

        // The sound table is kept, its item ids written in full; the
        // chapters that named a table left out, or none at all, go too.
        let kept: Vec<&str> = loaded.file.reward_tables.iter().map(|table| table.id.as_str()).collect();
        assert_eq!(kept, ["good"]);
        assert_eq!(
            loaded.file.reward_tables[0].entries[0].kind,
            RewardKind::Item { item: "minecraft:apple".into(), count: 1, max_count: None }
        );
        assert_eq!(ids(&loaded), ["a"]);

        let found = messages(&loaded);
        assert_eq!(
            found[..6],
            [
                "warning: reward_tables/1.json: entry #2: gives nothing",
                "error: reward_tables/2.json: the reward table has no `entries`",
                "error: reward_tables/3.json: another reward table already has the id `good`",
                "error: reward_tables/4.json: entry #1: an entry cannot be another table",
                "error: reward_tables/4.json: entry #2: `No Item` is not an item id",
                "error: reward_tables/5.json: every entry has `weight` 0: nothing can be drawn",
            ]
        );
        assert!(found[6].starts_with("error: reward_tables/6.json: not a valid reward table: "), "{}", found[6]);
        assert_eq!(
            found[7..],
            [
                "error: chapters/b.json > q: reward `r`: unknown reward table `empty`",
                "error: chapters/c.json > q: reward `r`: unknown reward table `nowhere`",
            ]
        );
    }

    #[test]
    fn the_later_tasks_and_rewards_are_checked() {
        let quest = r#", "repeat_cooldown": 5,
            "tasks": [ { "id": "a", "type": "xp_levels", "levels": 0 },
                       { "id": "b", "type": "dimension", "dimension": "The Nether" },
                       { "id": "c", "type": "advancement", "advancement": "Mine Stone" },
                       { "id": "d", "type": "location", "dimension": "Over World", "from": [0, 0, 0], "to": [1, 1, 1] },
                       { "id": "e", "type": "craft", "item": "bread", "count": 0 } ],
            "rewards": [ { "id": "r", "type": "command", "command": "  " },
                         { "id": "s", "type": "command", "command": "say a\nsay b" },
                         { "id": "t", "type": "choice", "table": "picks", "auto_claim": "enabled" } ]"#;
        let picks = r#"{ "id": "picks", "entries": [ { "type": "xp", "points": 1 } ] }"#;
        let loaded = load_pack(None, &[("a.json", &chapter("a", &[("q", &[], quest)]))], &[("picks.json", picks)]);
        assert!(loaded.file.chapters.is_empty());
        let expected = [
            "error: chapters/a.json > q: task `a`: `levels` must be at least 1",
            "error: chapters/a.json > q: task `b`: `The Nether` is not a dimension id",
            "error: chapters/a.json > q: task `c`: `Mine Stone` is not an advancement id",
            "error: chapters/a.json > q: task `d`: `Over World` is not a dimension id",
            "error: chapters/a.json > q: task `e`: `count` must be at least 1",
            "warning: chapters/a.json > q: `repeat_cooldown` does nothing without `repeatable`",
            "error: chapters/a.json > q: reward `r`: the `command` is empty",
            "error: chapters/a.json > q: reward `s`: the `command` must be one line",
            "warning: chapters/a.json > q: reward `t`: a choice is never claimed automatically",
        ];
        assert_eq!(messages(&loaded), expected);
    }

    #[test]
    fn a_repeatable_quest_needs_a_task_that_is_used_up() {
        let load_quest = |extra: &str| load(None, &[("a.json", &chapter("a", &[("q", &[], extra)]))]);
        let refused = ["error: chapters/a.json > q: a repeatable quest needs a task that is used up each time: \
                        a kill, a craft, a checkmark or a hand-in"];
        // Holding items and being somewhere stay true: the quest would
        // complete again at once, for ever. So would one without tasks.
        let idle = r#", "repeatable": true, "tasks": [ { "id": "t", "type": "item", "item": "apple" },
            { "id": "u", "type": "xp_levels", "levels": 1 }, { "id": "v", "type": "dimension", "dimension": "the_end" } ]"#;
        assert_eq!(messages(&load_quest(idle)), refused);
        assert_eq!(messages(&load_quest(r#", "repeatable": true"#)), refused);
        for used_up in [
            r#"{ "id": "t", "type": "item", "item": "apple", "consume": true }"#,
            r#"{ "id": "t", "type": "kill", "entity": "zombie" }"#,
            r#"{ "id": "t", "type": "craft", "item": "bread" }"#,
            r#"{ "id": "t", "type": "checkmark" }"#,
        ] {
            let quest = format!(r#", "repeatable": true, "repeat_cooldown": 60, "tasks": [ {used_up} ]"#);
            assert_eq!(load_quest(&quest).problems, [], "{used_up}");
        }
    }

    #[test]
    fn places_and_advancements_are_written_in_full() {
        let text = r#"{ "id": "a", "title": "A", "quests": [ { "id": "q", "title": "t",
            "tasks": [ { "id": "a", "type": "dimension", "dimension": "the_nether" },
                       { "id": "b", "type": "advancement", "advancement": "story/mine_stone" },
                       { "id": "c", "type": "location", "dimension": "overworld", "from": [10, 60, -5], "to": [-10, 70, -9] } ] } ] }"#;
        let loaded = load(None, &[("a.json", text)]);
        assert_eq!(loaded.problems, []);
        let tasks = &loaded.file.chapters[0].quests[0].tasks;
        assert_eq!(tasks[0].kind, TaskKind::Dimension { dimension: "minecraft:the_nether".into() });
        assert_eq!(tasks[1].kind, TaskKind::Advancement { advancement: "minecraft:story/mine_stone".into() });
        // A box's corners are written low then high.
        assert_eq!(
            tasks[2].kind,
            TaskKind::Location { dimension: Some("minecraft:overworld".into()), from: [-10, 60, -9], to: [10, 70, -5] }
        );
    }

    #[test]
    fn ids_are_written_in_full() {
        let text = r#"{ "id": "a", "title": "A", "icon": "book", "quests": [ { "id": "q", "title": "t", "icon": "create:gear",
            "tasks": [ { "id": "t", "type": "kill", "entity": "zombie" } ],
            "rewards": [ { "id": "r", "type": "item", "item": "apple" } ] } ] }"#;
        let loaded = load(None, &[("a.json", text)]);
        assert_eq!(loaded.problems, []);
        let chapter = &loaded.file.chapters[0];
        let quest = &chapter.quests[0];
        assert_eq!(chapter.icon.as_deref(), Some("minecraft:book"));
        assert_eq!(quest.icon.as_deref(), Some("create:gear"));
        assert_eq!(quest.tasks[0].kind, TaskKind::Kill { entity: "minecraft:zombie".into(), count: 1 });
        assert_eq!(quest.rewards[0].kind, RewardKind::Item { item: "minecraft:apple".into(), count: 1, max_count: None });
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
