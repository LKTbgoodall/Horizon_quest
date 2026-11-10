// `/quests` on the server: what the command line asks, who may ask it, and
// the lines the server answers with (help, the pack's problems, what a
// player just earned). The glue registers `USAGES`, hands each line to
// `parse`, and carries the command out. Pure: no game types.
use super::hq_book::titled;
use super::hq_logic::Book;
use super::hq_model::RewardKind;
use super::hq_progress::Effect;
use super::hq_validate::{Loaded, Severity};

/// The command's name, without the slash.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub const NAME: &str = "quests";

/// Every form of the command, as Horizon registers them, with the op level
/// it needs (0: everyone) and what it does. Ids are full ids and contain
/// `/`, so they are `text` arguments, which take the rest of the line.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub const USAGES: [(&str, u8, &str); 13] = [
    ("quests", 0, "show this help"),
    ("quests progress", 0, "list your open quests and how far they are"),
    ("quests claim", 0, "claim every reward waiting for you"),
    ("quests book", 0, "get a quest book: right-click it to open your quests"),
    ("quests check <task:text>", 0, "tick a checkmark task (its quest's id is enough)"),
    ("quests check", 0, "list the checkmark tasks you can tick"),
    ("quests submit <task:text>", 0, "hand in the items of a task (its quest's id is enough)"),
    ("quests submit", 0, "list the tasks you can hand items in for"),
    ("quests reload", 2, "read the quest files again"),
    ("quests validate", 2, "list the problems in the quest files"),
    ("quests complete <player:word> <quest:text>", 2, "complete a quest for a player"),
    ("quests reset <player:word> <quest:text>", 2, "reset a quest for a player"),
    ("quests reset_all <player:word>", 3, "reset all of a player's progress"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Command {
    Help,
    Progress,
    Claim,
    Book,
    Check { task: String },
    Submit { task: String },
    Reload,
    Validate,
    Complete { player: String, quest: String },
    Reset { player: String, quest: String },
    ResetAll { player: String },
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Command {
    /// The op level the command needs.
    pub fn level(&self) -> u8 {
        match self {
            Command::Help | Command::Progress | Command::Claim | Command::Book | Command::Check { .. } | Command::Submit { .. } => 0,
            Command::Reload | Command::Validate | Command::Complete { .. } | Command::Reset { .. } => 2,
            Command::ResetAll { .. } => 3,
        }
    }
}

/// The command in `line` (as the server gets it, without the slash). None
/// when the line is not `/quests`; Err with what to tell the player when it
/// is, but is not a form of it.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn parse(line: &str) -> Option<Result<Command, String>> {
    let mut words = line.split_whitespace();
    if words.next() != Some(NAME) {
        return None;
    }
    let words: Vec<&str> = words.collect();
    let command = match words.as_slice() {
        [] | ["help"] => Command::Help,
        ["progress"] => Command::Progress,
        ["claim"] => Command::Claim,
        ["book"] => Command::Book,
        ["check"] => Command::Check { task: String::new() },
        ["submit"] => Command::Submit { task: String::new() },
        ["check", task] => Command::Check { task: (*task).to_owned() },
        ["submit", task] => Command::Submit { task: (*task).to_owned() },
        ["reload"] => Command::Reload,
        ["validate"] => Command::Validate,
        ["complete", player, quest] => Command::Complete { player: (*player).to_owned(), quest: (*quest).to_owned() },
        ["reset", player, quest] => Command::Reset { player: (*player).to_owned(), quest: (*quest).to_owned() },
        ["reset_all", player] => Command::ResetAll { player: (*player).to_owned() },
        [verb, ..] => {
            // A known verb with the wrong arguments gets its usage.
            let usage = USAGES.iter().find(|(usage, ..)| usage.split_whitespace().nth(1) == Some(verb));
            return Some(Err(match usage {
                Some((usage, ..)) => format!("Usage: /{}", plain(usage)),
                None => format!("Unknown command. Try /{NAME} help."),
            }));
        }
    };
    Some(Ok(command))
}

/// The command in `line` if a player of op level `permission` may run it.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn parse_for(line: &str, permission: u8) -> Option<Result<Command, String>> {
    Some(parse(line)?.and_then(|command| {
        if permission >= command.level() { Ok(command) } else { Err("You may not use that command.".to_owned()) }
    }))
}

/// `quests check <task:text>` → `quests check <task>`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn plain(usage: &str) -> String {
    usage
        .split_whitespace()
        .map(|word| match word.strip_prefix('<').and_then(|w| w.split_once(':')) {
            Some((name, _)) => format!("<{name}>"),
            None => word.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The help, for a player of op level `permission`: one line per form they
/// may use.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn help(permission: u8) -> Vec<String> {
    USAGES
        .iter()
        .filter(|(_, level, _)| permission >= *level)
        .map(|(usage, _, what)| format!("/{}: {what}", plain(usage)))
        .collect()
}

/// What `/quests validate` and `/quests reload` answer: a summary, then the
/// problems, at most `limit` of them.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn report(loaded: &Loaded, limit: usize) -> Vec<String> {
    let chapters = loaded.file.chapters.len();
    let quests: usize = loaded.file.chapters.iter().map(|chapter| chapter.quests.len()).sum();
    let errors = loaded.errors();
    let warnings = loaded.problems.len() - errors;
    let mut lines = vec![format!(
        "{chapters} chapter{}, {quests} quest{} loaded; {errors} error{}, {warnings} warning{}.",
        plural(chapters),
        plural(quests),
        plural(errors),
        plural(warnings)
    )];
    // Errors first: they are why a chapter is missing.
    let mut problems: Vec<_> = loaded.problems.iter().collect();
    problems.sort_by_key(|problem| problem.severity != Severity::Error);
    lines.extend(problems.iter().take(limit).map(|problem| problem.to_string()));
    if problems.len() > limit {
        lines.push(format!("... and {} more (see the log).", problems.len() - limit));
    }
    lines
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// What to tell the player about an effect, in chat. None for those that
/// need no line (a task done: the book shows it).
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn announce(book: &Book, effect: &Effect) -> Option<String> {
    match effect {
        Effect::TaskCompleted { .. } => None,
        Effect::QuestCompleted { quest } => {
            let n = book.find(quest)?;
            let found = book.quest(n);
            let title = if found.title.is_empty() { quest.as_str() } else { found.title.as_str() };
            Some(if found.rewards.is_empty() {
                format!("Quest completed: {title}")
            } else {
                format!("Quest completed: {title} (rewards in the quest book)")
            })
        }
        Effect::ChapterCompleted { chapter } => {
            let found = book.file().chapters.iter().find(|c| c.id == *chapter)?;
            Some(format!("Chapter completed: {}", if found.title.is_empty() { chapter } else { &found.title }))
        }
        // A toast reward is its own message.
        Effect::Reward { kind: RewardKind::Toast { .. }, .. } => None,
        Effect::Reward { title, kind, .. } => Some(format!("Reward: {}", titled(title.as_deref(), kind))),
        Effect::QuestStartedOver { quest } => {
            let n = book.find(quest)?;
            let found = book.quest(n);
            let title = if found.title.is_empty() { quest.as_str() } else { found.title.as_str() };
            Some(format!("You can do this quest again: {title}"))
        }
        Effect::Refused(refusal) => Some(refusal.text().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::hq_progress::Refusal;
    use super::super::hq_validate::load;
    use super::*;

    #[test]
    fn every_form_parses() {
        let ok = |line: &str| parse(line).unwrap().unwrap();
        assert_eq!(ok("quests"), Command::Help);
        assert_eq!(ok("quests help"), Command::Help);
        assert_eq!(ok("  quests   claim "), Command::Claim);
        assert_eq!(ok("quests book"), Command::Book);
        assert_eq!(ok("quests check a/q/t"), Command::Check { task: "a/q/t".into() });
        assert_eq!(ok("quests submit a/q/t"), Command::Submit { task: "a/q/t".into() });
        assert_eq!(ok("quests reload"), Command::Reload);
        assert_eq!(ok("quests validate"), Command::Validate);
        assert_eq!(ok("quests complete Steve a/q"), Command::Complete { player: "Steve".into(), quest: "a/q".into() });
        assert_eq!(ok("quests reset Steve a/q"), Command::Reset { player: "Steve".into(), quest: "a/q".into() });
        assert_eq!(ok("quests reset_all Steve"), Command::ResetAll { player: "Steve".into() });
    }

    #[test]
    fn other_lines_and_bad_forms() {
        let ok = |line: &str| parse(line).unwrap().unwrap();
        for line in ["", "quest", "questsx claim", "say quests", "q"] {
            assert_eq!(parse(line), None, "{line}");
        }
        let err = |line: &str| parse(line).unwrap().unwrap_err();
        assert_eq!(err("quests dance"), "Unknown command. Try /quests help.");
        assert_eq!(ok("quests check"), Command::Check { task: String::new() });
        assert_eq!(ok("quests submit"), Command::Submit { task: String::new() });
        assert_eq!(err("quests check a b"), "Usage: /quests check <task>");
        assert_eq!(err("quests complete Steve"), "Usage: /quests complete <player> <quest>");
        assert_eq!(err("quests claim now"), "Usage: /quests claim");
        assert_eq!(err("quests help me"), "Unknown command. Try /quests help.");
    }

    #[test]
    fn usages_and_levels_agree() {
        // Each registered form parses, with placeholders for its arguments, to a command of its level.
        for (usage, level, what) in USAGES {
            let line: Vec<&str> = usage.split_whitespace().map(|word| if word.starts_with('<') { "x" } else { word }).collect();
            let command = parse(&line.join(" ")).unwrap().unwrap_or_else(|e| panic!("{usage}: {e}"));
            assert_eq!(command.level(), level, "{usage}");
            assert!(usage.starts_with(NAME) && !what.is_empty());
        }
    }

    #[test]
    fn permissions() {
        assert_eq!(parse_for("quests claim", 0), Some(Ok(Command::Claim)));
        assert_eq!(parse_for("quests reload", 1), Some(Err("You may not use that command.".into())));
        assert_eq!(parse_for("quests reload", 2), Some(Ok(Command::Reload)));
        assert_eq!(parse_for("quests reset_all Steve", 2), Some(Err("You may not use that command.".into())));
        assert_eq!(parse_for("quests reset_all Steve", 4), Some(Ok(Command::ResetAll { player: "Steve".into() })));
        assert_eq!(parse_for("say hi", 4), None);
        // A bad form says so before anything about permissions.
        assert_eq!(parse_for("quests dance", 0), Some(Err("Unknown command. Try /quests help.".into())));
    }

    #[test]
    fn help_lists_what_the_player_may_use() {
        assert_eq!(
            help(0),
            [
                "/quests: show this help",
                "/quests progress: list your open quests and how far they are",
                "/quests claim: claim every reward waiting for you",
                "/quests book: get a quest book: right-click it to open your quests",
                "/quests check <task>: tick a checkmark task (its quest's id is enough)",
                "/quests check: list the checkmark tasks you can tick",
                "/quests submit <task>: hand in the items of a task (its quest's id is enough)",
                "/quests submit: list the tasks you can hand items in for",
            ]
        );
        assert_eq!(help(2).len(), 12);
        assert_eq!(help(4).len(), USAGES.len());
        assert_eq!(help(4).last().unwrap(), "/quests reset_all <player>: reset all of a player's progress");
    }

    #[test]
    fn reports() {
        let good = r#"{ "id": "a", "title": "A", "quests": [ { "id": "one", "title": "One" } ] }"#;
        let warned = r#"{ "id": "b", "quests": [ { "id": "one" }, { "id": "two", "x": 1 } ] }"#;
        let loaded = load(None, &[("a.json", good)]);
        assert_eq!(report(&loaded, 5), ["1 chapter, 1 quest loaded; 0 errors, 0 warnings."]);

        let loaded = load(None, &[("a.json", good), ("b.json", warned), ("c.json", "oops")]);
        let lines = report(&loaded, 2);
        assert_eq!(lines[0], "2 chapters, 3 quests loaded; 1 error, 3 warnings.");
        // Errors lead, then warnings, up to the limit.
        assert!(lines[1].starts_with("error: chapters/c.json:"), "{lines:?}");
        assert!(lines[2].starts_with("warning: chapters/b.json"));
        assert_eq!(lines[3], "... and 2 more (see the log).");
        assert_eq!(report(&loaded, 10).len(), 5);
    }

    #[test]
    fn effects_are_announced() {
        let chapter = r#"{ "id": "a", "title": "Alpha", "quests": [
            { "id": "one", "title": "First steps", "rewards": [ { "id": "r", "type": "item", "item": "apple", "count": 2 } ] },
            { "id": "two", "x": 1 } ] }"#;
        let loaded = load(None, &[("a.json", chapter)]);
        let book = Book::new(loaded.file);
        let say = |effect: Effect| announce(&book, &effect);
        assert_eq!(say(Effect::TaskCompleted { task: "a/one/t".into() }), None);
        assert_eq!(
            say(Effect::QuestCompleted { quest: "a/one".into() }).as_deref(),
            Some("Quest completed: First steps (rewards in the quest book)")
        );
        // No title: the id; no reward: no hint.
        assert_eq!(say(Effect::QuestCompleted { quest: "a/two".into() }).as_deref(), Some("Quest completed: a/two"));
        assert_eq!(say(Effect::QuestCompleted { quest: "a/gone".into() }), None);
        assert_eq!(say(Effect::ChapterCompleted { chapter: "a".into() }).as_deref(), Some("Chapter completed: Alpha"));
        let reward = book.quest(0).rewards[0].kind.clone();
        assert_eq!(say(Effect::Reward { reward: "a/one/r".into(), title: None, kind: reward }).as_deref(), Some("Reward: 2x Apple"));
        assert_eq!(say(Effect::Refused(Refusal::NotOpen)).as_deref(), Some("That quest is not open yet."));

        // What a table gave is named by its entry: its title, else its kind.
        let cake = RewardKind::Item { item: "minecraft:cake".into(), count: 1 };
        let drawn = |title: Option<&str>| Effect::Reward { reward: "a/one/r".into(), title: title.map(str::to_owned), kind: cake.clone() };
        assert_eq!(say(drawn(Some("A cake!"))).as_deref(), Some("Reward: A cake!"));
        assert_eq!(say(drawn(None)).as_deref(), Some("Reward: Cake"));
        // A toast is its own message.
        let toast = RewardKind::Toast { text: "Mind the lava.".into() };
        assert_eq!(say(Effect::Reward { reward: "a/one/r".into(), title: Some("Welcome".into()), kind: toast }), None);
        assert_eq!(
            say(Effect::QuestStartedOver { quest: "a/one".into() }).as_deref(),
            Some("You can do this quest again: First steps")
        );
        assert_eq!(
            say(Effect::QuestStartedOver { quest: "a/two".into() }).as_deref(),
            Some("You can do this quest again: a/two")
        );
        assert_eq!(say(Effect::QuestStartedOver { quest: "a/gone".into() }), None);
        assert_eq!(
            say(Effect::Refused(Refusal::NeedsChoice)).as_deref(),
            Some("Pick that reward in the quest book: it lets you choose.")
        );
    }
}
