//! The example pack (`built-in-pack/`, compiled in as hq_built_in): it must load
//! without a single problem, and a player must be able to finish it.

use crate::hq_logic::{Book, QuestState};
use crate::hq_model::{Progress, RewardKind, TaskKind};
use crate::hq_progress::{
    Actor, Effect, QuestEvent, apply, submit_request, wanted_crafts, wanted_items,
};
use crate::hq_validate::load_dir;
use std::path::Path;

fn example() -> Book {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("built-in-pack");
    let loaded = load_dir(&dir);
    let problems: Vec<String> = loaded.problems.iter().map(|p| p.to_string()).collect();
    assert_eq!(problems, Vec::<String>::new());
    Book::new(loaded.file)
}

#[test]
fn the_example_pack_is_sound() {
    let book = example();
    let chapters: Vec<&str> = book.file().chapters.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        chapters,
        [
            "getting_started",
            "further_afield",
            "iron_age",
            "diamonds",
            "the_nether",
            "the_end",
            "beyond",
            "farming",
            "monsters",
            "wide_world",
            "caves",
            "oceans",
            "new_horizons",
            "trial_chambers",
            "archaeology",
            "nether_frontier",
            "animals",
            "kitchen",
            "villagers",
            "arsenal",
            "redstone",
            "builder",
            "magic",
            "collector",
        ]
    );
    let sizes: Vec<usize> = (0..chapters.len()).map(|c| book.chapter_nodes(c).len()).collect();
    assert_eq!(sizes, [16, 9, 14, 15, 14, 11, 16, 15, 16, 18, 13, 13, 20, 15, 10, 13, 15, 9, 11, 8, 13, 19, 11, 10]);
    assert_eq!(sizes.iter().sum::<usize>(), 324);
    // The main story, from the first tree to the dragon and beyond, then the other paths.
    let groups: Vec<(&str, Vec<&str>)> = book
        .file()
        .settings
        .groups
        .iter()
        .map(|g| {
            let members = book.file().chapters.iter().filter(|c| c.group.as_deref() == Some(g.id.as_str())).map(|c| c.id.as_str());
            (g.title.as_str(), members.collect())
        })
        .collect();
    assert_eq!(
        groups,
        [
            ("Main story", vec!["getting_started", "iron_age", "diamonds", "the_nether", "the_end", "beyond"]),
            (
                "Exploration",
                vec![
                    "further_afield",
                    "wide_world",
                    "caves",
                    "oceans",
                    "new_horizons",
                    "trial_chambers",
                    "archaeology",
                    "nether_frontier",
                ]
            ),
            ("Life and farming", vec!["farming", "animals", "kitchen", "villagers"]),
            ("Combat", vec!["monsters", "arsenal"]),
            ("Crafting and building", vec!["redstone", "builder", "magic"]),
            ("Challenges", vec!["collector"]),
        ]
    );
    let tables: Vec<&str> = book.file().reward_tables.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        tables,
        [
            "baker_loot",
            "builder_choice",
            "explorer_loot",
            "farmer_loot",
            "hunter_loot",
            "seasoned_choice",
            "treasure_choice"
        ]
    );

    // A new player sees every quest but the surprises that end a chapter,
    // and only the first is open.
    let statuses = book.statuses(&Progress::default(), "steve");
    let hidden: Vec<&str> =
        (0..statuses.len()).filter(|&n| !statuses[n].visible).map(|n| book.nodes()[n].id.as_str()).collect();
    assert_eq!(
        hidden,
        [
            "getting_started/settled",
            "the_nether/nether_done",
            "the_end/credits",
            "beyond/master",
            "monsters/hunter",
            "wide_world/traveller",
            "oceans/sea_master",
            "trial_chambers/trials_done",
            "animals/animal_lover",
            "redstone/engineer",
            "builder/master_builder",
            "magic/arcanist",
            "collector/completionist",
        ]
    );
    let open: Vec<&str> = (0..statuses.len())
        .filter(|&n| statuses[n].state != QuestState::Locked)
        .map(|n| book.nodes()[n].id.as_str())
        .collect();
    assert_eq!(open, ["getting_started/welcome"]);
}

#[test]
fn the_example_pack_can_be_finished() {
    let book = example();
    let mut progress = Progress::default();
    let mut effects = Vec::new();
    let mut now = 0;
    let mut play = |progress: &mut Progress, event: QuestEvent| {
        now += 1;
        effects.extend(apply(&book, progress, &Actor::new("steve", now).with_roll(now), &event).effects);
    };

    // Do each quest's tasks the way the glue would report them, dependencies first.
    for &n in book.order() {
        let quest = book.quest(n);
        for (t, task) in quest.tasks.iter().enumerate() {
            let id = book.task_id(n, t);
            match &task.kind {
                TaskKind::Item { consume: false, .. } => {
                    assert!(wanted_items(&book, &progress).iter().any(|w| w.task == id), "{id} is not wanted");
                    play(&mut progress, QuestEvent::ItemCount { task: id, count: task.kind.goal() });
                }
                TaskKind::Item { consume: true, .. } => {
                    let wanted = submit_request(&book, &progress, &id).unwrap();
                    play(&mut progress, QuestEvent::Submitted { task: id, count: wanted.missing });
                }
                TaskKind::Kill { entity, count } => {
                    for _ in 0..*count {
                        play(&mut progress, QuestEvent::Killed { entity: entity.clone() });
                    }
                }
                TaskKind::Checkmark => play(&mut progress, QuestEvent::Check { task: id }),
                TaskKind::Craft { .. } => {
                    assert!(wanted_crafts(&book, &progress).iter().any(|w| w.task == id), "{id} is not wanted");
                    play(&mut progress, QuestEvent::Crafted { task: id, count: task.kind.goal() });
                }
                TaskKind::XpLevels { levels } => play(&mut progress, QuestEvent::Level { level: *levels }),
                TaskKind::Dimension { dimension } => {
                    play(&mut progress, QuestEvent::At { dimension: dimension.clone(), position: [0, 64, 0] });
                }
                TaskKind::Location { dimension, from, .. } => {
                    let dimension = dimension.clone().unwrap_or_else(|| "minecraft:overworld".to_owned());
                    play(&mut progress, QuestEvent::At { dimension, position: *from });
                }
                TaskKind::Advancement { advancement } => {
                    play(&mut progress, QuestEvent::Advancement { advancement: advancement.clone() });
                }
            }
        }
        assert!(progress.is_completed(&book.nodes()[n].id), "{} did not complete", book.nodes()[n].id);
    }
    // Choices are the player's to make, in the book; the rest comes at once.
    for n in 0..book.nodes().len() {
        for (r, reward) in book.quest(n).rewards.iter().enumerate() {
            if matches!(reward.kind, RewardKind::Choice { .. }) {
                play(&mut progress, QuestEvent::Claim { reward: book.reward_id(n, r), choice: Some(0) });
            }
        }
    }
    play(&mut progress, QuestEvent::ClaimAll);

    for chapter in book.file().chapters.iter().map(|c| c.id.as_str()) {
        let done = Effect::ChapterCompleted { chapter: chapter.into() };
        assert_eq!(effects.iter().filter(|e| **e == done).count(), 1, "{chapter}");
    }
    assert!(!effects.iter().any(|e| matches!(e, Effect::Refused(_))), "{effects:?}");
    let rewards = effects.iter().filter(|e| matches!(e, Effect::Reward { .. })).count();
    assert_eq!(rewards, book.nodes().iter().enumerate().map(|(n, _)| book.quest(n).rewards.len()).sum::<usize>());
    // What a table gave is one of its entries, never the table itself.
    assert!(!effects.iter().any(|e| matches!(e, Effect::Reward { kind, .. } if kind.table().is_some())));
    // Everything shows as done, but a repeatable quest whose cooldown ran out
    // during the run (one step is a second here): it has started over.
    let statuses = book.statuses(&progress, "steve");
    for (n, status) in statuses.iter().enumerate() {
        let started_over = book.quest(n).repeatable && status.state == QuestState::InProgress;
        assert!(status.visible && (status.state == QuestState::Done || started_over), "{}", book.nodes()[n].id);
    }
}
