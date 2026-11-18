// The mod's settings files. Horizon writes the default text when a file is
// missing and hands its text over (API 397); this module reads it. Pure: no
// game types, so `cargo test` runs it.
// - `config/horizon_quests-server.toml`, read by the server for each world (a
//   world can keep its own in `<world>/serverconfig/`).
// - `config/horizon_quests-client.toml`, read by the window: what only this
//   player sees.
// A wrong or unknown setting keeps its default and is reported; saved changes
// apply at once.

/// The server file's text when it is missing.
pub const DEFAULT_TEXT: &str = "\
# Horizon Quests: server settings.
# A world can have its own copy in <world>/serverconfig/horizon_quests-server.toml.
# Saved changes apply at once, as /quests reload does.

# Use the built-in example chapters (\"Getting started\", \"Further afield\") when
# the world has no quests of its own (<world>/horizon-quests/quests/).
# false: no quests at all.
example_chapter = true

# Give each player a quest book the first time they join this world (a
# right-click with it opens the book, as the key does). /quests book gives
# another one at any time.
quest_book = true

# Let players start and join parties (/quests team ...), which share one
# progress. false: no new party and no joining; members can still leave.
parties = true

# The most members a party can have, from 2 to 100. 0: no limit.
party_size = 8

# Ticks between two looks at a player's inventory for item tasks (20 ticks
# is one second), from 1 to 1200. 0: what the quest pack says (20 unless it
# sets detection_delay). Raise it on a busy server.
detection_delay = 0

# Say \"Quest completed\" and \"Chapter completed\" in chat. false: the quest
# book and the rewards still show it.
completion_messages = true
";

/// The client file's text when it is missing.
pub const CLIENT_TEXT: &str = "\
# Horizon Quests: settings for this player only.
# Saved changes apply at once.

# Show the boxes some rewards pop up in the top-right corner of the screen.
toasts = true

# How long each of those boxes stays, in seconds, from 1 to 30.
toast_seconds = 5
";

/// What the server file says, each setting at its default when missing or
/// wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub example_chapter: bool,
    pub quest_book: bool,
    pub parties: bool,
    /// 0: no limit.
    pub party_size: u32,
    /// 0: the pack's own.
    pub detection_delay: u32,
    pub completion_messages: bool,
}

impl ServerConfig {
    pub const DEFAULT: Self =
        Self { example_chapter: true, quest_book: true, parties: true, party_size: 8, detection_delay: 0, completion_messages: true };
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What the client file says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientConfig {
    pub toasts: bool,
    pub toast_seconds: u32,
}

impl ClientConfig {
    pub const DEFAULT: Self = Self { toasts: true, toast_seconds: 5 };

    /// How long a toast stays, in window ticks (20 a second).
    pub fn toast_ticks(self) -> u32 {
        self.toast_seconds * 20
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One setting read into its place, or a line saying why not.
struct Reader<'a> {
    problems: &'a mut Vec<String>,
}

impl Reader<'_> {
    fn bool(&mut self, key: &str, value: &toml::Value, into: &mut bool) {
        match value.as_bool() {
            Some(on) => *into = on,
            None => self.problems.push(format!("`{key}` must be true or false, not {value}")),
        }
    }

    /// A whole number in `range`, or `zero_ok` for 0.
    fn number(&mut self, key: &str, value: &toml::Value, range: std::ops::RangeInclusive<u32>, zero_ok: bool, into: &mut u32) {
        let found = value.as_integer().and_then(|n| u32::try_from(n).ok()).filter(|n| range.contains(n) || (zero_ok && *n == 0));
        match found {
            Some(n) => *into = n,
            None => {
                let zero = if zero_ok { " or 0" } else { "" };
                self.problems.push(format!("`{key}` must be a whole number from {} to {}{zero}, not {value}", range.start(), range.end()));
            }
        }
    }
}

/// The file's text as a table, or the problem that keeps every default.
fn table(text: &str, problems: &mut Vec<String>) -> Option<toml::Table> {
    match text.trim_start_matches('\u{feff}').parse::<toml::Table>() {
        Ok(table) => Some(table),
        Err(e) => {
            problems.push(format!("not valid TOML, every setting stays at its default: {}", e.message()));
            None
        }
    }
}

/// Reads the server file's text: the settings, and a line for each problem
/// (the setting it concerns keeps its default).
pub fn parse(text: &str) -> (ServerConfig, Vec<String>) {
    let mut config = ServerConfig::default();
    let mut problems = Vec::new();
    let Some(table) = table(text, &mut problems) else {
        return (config, problems);
    };
    let mut read = Reader { problems: &mut problems };
    for (key, value) in &table {
        match key.as_str() {
            "example_chapter" => read.bool(key, value, &mut config.example_chapter),
            "quest_book" => read.bool(key, value, &mut config.quest_book),
            "parties" => read.bool(key, value, &mut config.parties),
            "party_size" => read.number(key, value, 2..=100, true, &mut config.party_size),
            "detection_delay" => read.number(key, value, 1..=1200, true, &mut config.detection_delay),
            "completion_messages" => read.bool(key, value, &mut config.completion_messages),
            other => read.problems.push(format!("unknown setting `{other}`, ignored")),
        }
    }
    (config, problems)
}

/// Reads the client file's text, as `parse` does the server's.
pub fn parse_client(text: &str) -> (ClientConfig, Vec<String>) {
    let mut config = ClientConfig::default();
    let mut problems = Vec::new();
    let Some(table) = table(text, &mut problems) else {
        return (config, problems);
    };
    let mut read = Reader { problems: &mut problems };
    for (key, value) in &table {
        match key.as_str() {
            "toasts" => read.bool(key, value, &mut config.toasts),
            "toast_seconds" => read.number(key, value, 1..=30, false, &mut config.toast_seconds),
            other => read.problems.push(format!("unknown setting `{other}`, ignored")),
        }
    }
    (config, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_texts_read_as_the_defaults() {
        assert_eq!(parse(DEFAULT_TEXT), (ServerConfig::default(), Vec::new()));
        assert_eq!(parse(""), (ServerConfig::default(), Vec::new()));
        assert_eq!(parse_client(CLIENT_TEXT), (ClientConfig::default(), Vec::new()));
        assert_eq!(parse_client(""), (ClientConfig::default(), Vec::new()));
        assert_eq!(ClientConfig::default().toast_ticks(), 100);
    }

    #[test]
    fn the_example_chapter_can_be_turned_off() {
        let (config, problems) = parse("example_chapter = false\n");
        assert!(!config.example_chapter);
        assert!(problems.is_empty());
        // A byte-order mark, as Windows editors write.
        assert!(!parse("\u{feff}example_chapter = false").0.example_chapter);
    }

    #[test]
    fn the_quest_book_can_be_turned_off() {
        let (config, problems) = parse("quest_book = false\n");
        assert_eq!(config, ServerConfig { quest_book: false, ..ServerConfig::DEFAULT });
        assert!(problems.is_empty());
        assert!(parse("quest_book = 1").1[0].contains("true or false"));
    }

    #[test]
    fn parties_messages_and_the_delay() {
        let text = "parties = false\nparty_size = 3\ndetection_delay = 100\ncompletion_messages = false\n";
        let (config, problems) = parse(text);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            config,
            ServerConfig { parties: false, party_size: 3, detection_delay: 100, completion_messages: false, ..ServerConfig::DEFAULT }
        );
        // 0 means no limit, or the pack's delay.
        assert_eq!(parse("party_size = 0\ndetection_delay = 0").0, ServerConfig { party_size: 0, ..ServerConfig::DEFAULT });
        for bad in ["party_size = 1", "party_size = 101", "party_size = -2", "party_size = 2.5", "detection_delay = 1201"] {
            let (config, problems) = parse(bad);
            assert_eq!(config, ServerConfig::DEFAULT, "{bad}");
            assert!(problems[0].contains("whole number from"), "{bad}: {problems:?}");
        }
        assert_eq!(parse("party_size = 1").1[0], "`party_size` must be a whole number from 2 to 100 or 0, not 1");
    }

    #[test]
    fn toasts_can_be_turned_off_or_kept_longer() {
        let (config, problems) = parse_client("toasts = false\ntoast_seconds = 12\n");
        assert!(problems.is_empty());
        assert_eq!(config, ClientConfig { toasts: false, toast_seconds: 12 });
        assert_eq!(config.toast_ticks(), 240);
        let (config, problems) = parse_client("toast_seconds = 0\ncolour = 1\n");
        assert_eq!(config, ClientConfig::DEFAULT);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().any(|p| p == "`toast_seconds` must be a whole number from 1 to 30, not 0"));
        assert!(parse_client("toasts = ").1[0].starts_with("not valid TOML"));
    }

    #[test]
    fn mistakes_keep_the_default_and_say_why() {
        let (config, problems) = parse("example_chapter = \"no\"\nexmaple = 1\n");
        assert!(config.example_chapter);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("true or false") || problems[1].contains("true or false"));
        assert!(problems.iter().any(|p| p.contains("unknown setting `exmaple`")));

        let (config, problems) = parse("example_chapter = fals");
        assert!(config.example_chapter);
        assert!(problems[0].starts_with("not valid TOML"), "{problems:?}");
    }
}
