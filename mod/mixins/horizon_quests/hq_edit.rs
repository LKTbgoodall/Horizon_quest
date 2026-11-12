// Edit mode's changes to a quest pack: an operator's book asks for one
// `Edit` at a time, the server applies it to the pack in use, checks the
// result as the loader would (hq_validate) and saves the whole pack. The
// book applies the same edit to its own copy at once, so a moved quest does
// not jump back while the server answers; the server's pack then replaces
// it. Pure: no game types.
//
// Ids stay as they are once made: progress files refer to full ids. A new
// quest gets a fresh id from its title (`new_quest`, `new_quest_2`...).
use super::hq_model::{DependencyMode, Quest, QuestFile, Shape, Task, TaskKind, Visibility, dependency_id, quest_id};
use super::hq_validate::{self as validate, Severity};
use serde::{Deserialize, Serialize};

/// Where the pack in use comes from (`ToClient::Editor`): the world's own
/// folder, the modpack's (`config/horizon_quests/quests/`), the data packs,
/// the pack built into the mod, or nowhere.
pub const SOURCE_WORLD: &str = "world";
pub const SOURCE_MODPACK: &str = "modpack";
pub const SOURCE_DATA_PACKS: &str = "data_packs";
pub const SOURCE_BUILT_IN: &str = "built_in";
pub const SOURCE_NONE: &str = "none";

/// The op level that may edit the quests, as `/quests reload` needs.
pub const EDIT_LEVEL: u8 = 2;

/// The title of a quest made in the book.
pub const NEW_QUEST: &str = "New quest";
/// How far from the canvas's middle a quest may be put, in grid cells.
const FAR: f32 = 10_000.0;

/// One change to the pack. Quests are named by full id (`chapter/quest`),
/// places in grid cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    /// A new quest in chapter `chapter` (its id) at (`x`, `y`): one
    /// checkmark task, so it does not complete on its own.
    NewQuest { chapter: String, x: f32, y: f32 },
    MoveQuest { quest: String, x: f32, y: f32 },
    /// A copy of the quest, with its tasks, rewards and dependencies, at
    /// (`x`, `y`) in the same chapter.
    DuplicateQuest { quest: String, x: f32, y: f32 },
    /// Removes the quest, and every other quest's dependency on it.
    DeleteQuest { quest: String },
    /// Makes `quest` need `needs` (`on`), or stop needing it.
    Link { quest: String, needs: String, on: bool },
    /// The quest form's fields: everything but the place, the links, the
    /// tasks and the rewards.
    SetQuestInfo { quest: String, info: QuestInfo },
}

/// What the quest form edits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuestInfo {
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Vec<String>,
    pub icon: Option<String>,
    pub shape: Option<Shape>,
    pub size: Option<f32>,
    pub visibility: Visibility,
    pub dependency_mode: DependencyMode,
    pub min_required: Option<u32>,
    pub optional: bool,
    pub sequential_tasks: bool,
    pub repeatable: bool,
    pub repeat_cooldown: u32,
}

impl QuestInfo {
    pub fn of(quest: &Quest) -> Self {
        Self {
            title: quest.title.clone(),
            subtitle: quest.subtitle.clone(),
            description: quest.description.clone(),
            icon: quest.icon.clone(),
            shape: quest.shape,
            size: quest.size,
            visibility: quest.visibility,
            dependency_mode: quest.dependency_mode,
            min_required: quest.min_required,
            optional: quest.optional,
            sequential_tasks: quest.sequential_tasks,
            repeatable: quest.repeatable,
            repeat_cooldown: quest.repeat_cooldown,
        }
    }

    fn write(&self, quest: &mut Quest) {
        quest.title = self.title.clone();
        quest.subtitle = self.subtitle.clone();
        quest.description = self.description.clone();
        quest.icon = self.icon.clone();
        quest.shape = self.shape;
        quest.size = self.size;
        quest.visibility = self.visibility;
        quest.dependency_mode = self.dependency_mode;
        quest.min_required = self.min_required;
        quest.optional = self.optional;
        quest.sequential_tasks = self.sequential_tasks;
        quest.repeatable = self.repeatable;
        quest.repeat_cooldown = self.repeat_cooldown;
    }
}

/// The chapter and place of quest `id` (a full id).
fn locate(file: &QuestFile, id: &str) -> Option<(usize, usize)> {
    let (chapter, quest) = id.split_once('/')?;
    let c = file.chapters.iter().position(|ch| ch.id == chapter)?;
    let q = file.chapters[c].quests.iter().position(|qu| qu.id == quest)?;
    Some((c, q))
}

fn place(x: f32, y: f32) -> Result<(f32, f32), String> {
    if x.is_finite() && y.is_finite() && x.abs() <= FAR && y.abs() <= FAR {
        Ok((x, y))
    } else {
        Err("That place is off the canvas.".to_owned())
    }
}

/// `base` in lower case with `_` for anything that cannot be in an id.
pub fn slug(base: &str) -> String {
    let mut id = String::new();
    for c in base.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            id.push(c);
        } else if !id.ends_with('_') && !id.is_empty() {
            id.push('_');
        }
    }
    let id = id.trim_end_matches('_');
    if id.is_empty() { "quest".to_owned() } else { id.chars().take(40).collect() }
}

/// An id made from `base` that no quest of chapter `c` has yet.
fn fresh_id(file: &QuestFile, c: usize, base: &str) -> String {
    let base = slug(base);
    let taken = |id: &str| file.chapters[c].quests.iter().any(|q| q.id == id);
    if !taken(&base) {
        return base;
    }
    (2..).map(|k| format!("{base}_{k}")).find(|id| !taken(id)).unwrap_or(base)
}

/// Applies `edit` to `file`. Returns the full id of the quest it made, if
/// any. The result is not checked yet: see `check`.
pub fn apply(file: &mut QuestFile, edit: &Edit) -> Result<Option<String>, String> {
    let missing = |id: &str| format!("There is no quest {id} any more.");
    match edit {
        Edit::NewQuest { chapter, x, y } => {
            let (x, y) = place(*x, *y)?;
            let c = file.chapters.iter().position(|ch| ch.id == *chapter).ok_or_else(|| format!("There is no chapter {chapter} any more."))?;
            let id = fresh_id(file, c, NEW_QUEST);
            let task = Task { id: "check".to_owned(), title: None, kind: TaskKind::Checkmark };
            file.chapters[c].quests.push(Quest { id: id.clone(), title: NEW_QUEST.to_owned(), x, y, tasks: vec![task], ..Quest::default() });
            Ok(Some(quest_id(chapter, &id)))
        }
        Edit::MoveQuest { quest, x, y } => {
            let (x, y) = place(*x, *y)?;
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let moved = &mut file.chapters[c].quests[q];
            (moved.x, moved.y) = (x, y);
            Ok(None)
        }
        Edit::DuplicateQuest { quest, x, y } => {
            let (x, y) = place(*x, *y)?;
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let mut copy = file.chapters[c].quests[q].clone();
            copy.id = fresh_id(file, c, &format!("{}_copy", copy.id));
            (copy.x, copy.y) = (x, y);
            let id = quest_id(&file.chapters[c].id, &copy.id);
            file.chapters[c].quests.insert(q + 1, copy);
            Ok(Some(id))
        }
        Edit::DeleteQuest { quest } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            file.chapters[c].quests.remove(q);
            for chapter in &mut file.chapters {
                let here = chapter.id.clone();
                for other in &mut chapter.quests {
                    other.dependencies.retain(|d| dependency_id(&here, d) != *quest);
                }
            }
            Ok(None)
        }
        Edit::Link { quest, needs, on } => {
            if quest == needs {
                return Err("A quest cannot need itself.".to_owned());
            }
            locate(file, needs).ok_or_else(|| missing(needs))?;
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let here = file.chapters[c].id.clone();
            let deps = &mut file.chapters[c].quests[q].dependencies;
            deps.retain(|d| dependency_id(&here, d) != *needs);
            if *on {
                // Same chapter: the short id, as pack authors write it.
                let short = needs.strip_prefix(&format!("{here}/")).unwrap_or(needs);
                deps.push(short.to_owned());
            }
            Ok(None)
        }
        Edit::SetQuestInfo { quest, info } => {
            if info.size.is_some_and(|size| !(0.25..=4.0).contains(&size)) {
                return Err("A quest's size goes from 0.25 to 4.".to_owned());
            }
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            info.write(&mut file.chapters[c].quests[q]);
            Ok(None)
        }
    }
}

/// `file` as the loader reads it back, when that leaves nothing out and
/// finds no error; else why not, in a line for the book.
pub fn check(file: &QuestFile) -> Result<QuestFile, String> {
    fn json<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap_or_default()
    }
    // Numbered so the loader keeps the chapters' order.
    let chapters: Vec<(String, String)> =
        file.chapters.iter().enumerate().map(|(i, chapter)| (format!("{i:03}.json"), json(chapter))).collect();
    let tables: Vec<(String, String)> = file.reward_tables.iter().map(|table| (format!("{}.json", table.id), json(table))).collect();
    fn as_refs(files: &[(String, String)]) -> Vec<(&str, &str)> {
        files.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect()
    }
    let loaded = validate::load_pack(Some(&json(&file.settings)), &as_refs(&chapters), &as_refs(&tables));
    if let Some(problem) = loaded.problems.iter().find(|p| p.severity == Severity::Error) {
        return Err(format!("Not saved: {} ({})", problem.message, problem.place));
    }
    if loaded.file.chapters.len() != file.chapters.len() || loaded.file.reward_tables.len() != file.reward_tables.len() {
        return Err("Not saved: the quests would not load back whole.".to_owned());
    }
    Ok(loaded.file)
}

/// Where a quest dropped at grid place `at` goes: the nearest half cell.
pub fn snap(at: [f32; 2]) -> [f32; 2] {
    [(at[0] * 2.0).round() / 2.0, (at[1] * 2.0).round() / 2.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::hq_model::Chapter;

    fn pack() -> QuestFile {
        let quest = |id: &str, deps: &[&str]| Quest {
            id: id.to_owned(),
            title: id.to_owned(),
            dependencies: deps.iter().map(|d| (*d).to_owned()).collect(),
            tasks: vec![Task { id: "t".to_owned(), title: None, kind: TaskKind::Checkmark }],
            ..Quest::default()
        };
        let one = Chapter { id: "one".to_owned(), title: "One".to_owned(), quests: vec![quest("a", &[]), quest("b", &["a"])], ..Chapter::default() };
        let two = Chapter { id: "two".to_owned(), title: "Two".to_owned(), order: 1, quests: vec![quest("c", &["one/b"])], ..Chapter::default() };
        QuestFile { chapters: vec![one, two], ..QuestFile::default() }
    }

    #[test]
    fn a_new_quest_gets_a_fresh_id_and_a_checkmark() {
        let mut file = pack();
        let first = apply(&mut file, &Edit::NewQuest { chapter: "one".into(), x: 2.0, y: 1.5 }).unwrap();
        let second = apply(&mut file, &Edit::NewQuest { chapter: "one".into(), x: 3.0, y: 1.5 }).unwrap();
        assert_eq!(first.as_deref(), Some("one/new_quest"));
        assert_eq!(second.as_deref(), Some("one/new_quest_2"));
        let made = &file.chapters[0].quests[2];
        assert_eq!((made.x, made.y, made.title.as_str()), (2.0, 1.5, NEW_QUEST));
        assert_eq!(made.tasks[0].kind, TaskKind::Checkmark);
        check(&file).unwrap();
    }

    #[test]
    fn moving_changes_only_the_place() {
        let mut file = pack();
        apply(&mut file, &Edit::MoveQuest { quest: "one/b".into(), x: -4.5, y: 2.0 }).unwrap();
        let b = &file.chapters[0].quests[1];
        assert_eq!((b.x, b.y), (-4.5, 2.0));
        assert!(apply(&mut file, &Edit::MoveQuest { quest: "one/zz".into(), x: 0.0, y: 0.0 }).is_err());
        assert!(apply(&mut file, &Edit::MoveQuest { quest: "one/a".into(), x: f32::NAN, y: 0.0 }).is_err());
    }

    #[test]
    fn a_copy_keeps_tasks_and_dependencies_under_a_new_id() {
        let mut file = pack();
        let id = apply(&mut file, &Edit::DuplicateQuest { quest: "one/b".into(), x: 5.0, y: 0.0 }).unwrap();
        assert_eq!(id.as_deref(), Some("one/b_copy"));
        let copy = &file.chapters[0].quests[2];
        assert_eq!(copy.dependencies, vec!["a".to_owned()]);
        assert_eq!(copy.tasks.len(), 1);
        check(&file).unwrap();
    }

    #[test]
    fn deleting_a_quest_drops_the_dependencies_on_it_in_every_chapter() {
        let mut file = pack();
        apply(&mut file, &Edit::DeleteQuest { quest: "one/b".into() }).unwrap();
        assert_eq!(file.chapters[0].quests.len(), 1);
        assert!(file.chapters[1].quests[0].dependencies.is_empty());
        check(&file).unwrap();
    }

    #[test]
    fn links_are_short_in_the_same_chapter_and_full_across() {
        let mut file = pack();
        apply(&mut file, &Edit::Link { quest: "one/a".into(), needs: "two/c".into(), on: true }).unwrap();
        assert_eq!(file.chapters[0].quests[0].dependencies, vec!["two/c".to_owned()]);
        // a needs c, c needs b, b needs a: a cycle the check refuses.
        assert!(check(&file).is_err());
        apply(&mut file, &Edit::Link { quest: "one/a".into(), needs: "two/c".into(), on: false }).unwrap();
        assert!(file.chapters[0].quests[0].dependencies.is_empty());
        apply(&mut file, &Edit::Link { quest: "two/c".into(), needs: "one/a".into(), on: true }).unwrap();
        assert_eq!(file.chapters[1].quests[0].dependencies, vec!["one/b".to_owned(), "one/a".to_owned()]);
        assert!(apply(&mut file, &Edit::Link { quest: "one/a".into(), needs: "one/a".into(), on: true }).is_err());
        check(&file).unwrap();
    }

    #[test]
    fn the_form_changes_the_info_and_keeps_tasks_links_and_place() {
        let mut file = pack();
        let mut info = QuestInfo::of(&file.chapters[0].quests[1]);
        info.title = "Bee".into();
        info.description = vec!["One".into(), "".into(), "Two".into()];
        info.shape = Some(Shape::Diamond);
        info.icon = Some("minecraft:stone".into());
        apply(&mut file, &Edit::SetQuestInfo { quest: "one/b".into(), info: info.clone() }).unwrap();
        let b = &file.chapters[0].quests[1];
        assert_eq!(QuestInfo::of(b), info);
        assert_eq!((b.dependencies.len(), b.tasks.len()), (1, 1));
        info.size = Some(9.0);
        assert!(apply(&mut file, &Edit::SetQuestInfo { quest: "one/b".into(), info }).is_err());
        check(&file).unwrap();
    }

    #[test]
    fn ids_come_from_titles() {
        assert_eq!(slug("New quest"), "new_quest");
        assert_eq!(slug("  Iron -- Age! "), "iron_age");
        assert_eq!(slug("!!!"), "quest");
        assert_eq!(snap([1.26, -0.74]), [1.5, -0.5]);
    }
}
