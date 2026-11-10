// The server's settings file, `config/horizon_quests-server.toml` (a world
// can keep its own in `<world>/serverconfig/`). Horizon writes the default
// text when the file is missing and hands its text over (API 397); this
// module reads it. Pure: no game types, so `cargo test` runs it.

/// The text written when the file is missing.
pub const DEFAULT_TEXT: &str = "\
# Horizon Quests: server settings.
# A world can have its own copy in <world>/serverconfig/horizon_quests-server.toml.
# Saved changes apply at once, as /quests reload does.

# Use the built-in example chapter \"Getting started\" when the world has no
# quests of its own (<world>/horizon-quests/quests/). false: no quests at all.
example_chapter = true
";

/// What the file says, each setting at its default when missing or wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub example_chapter: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { example_chapter: true }
    }
}

/// Reads the file's text: the settings, and a line for each problem (the
/// setting it concerns keeps its default).
pub fn parse(text: &str) -> (ServerConfig, Vec<String>) {
    let mut config = ServerConfig::default();
    let mut problems = Vec::new();
    let table = match text.trim_start_matches('\u{feff}').parse::<toml::Table>() {
        Ok(table) => table,
        Err(e) => {
            problems.push(format!("not valid TOML, every setting stays at its default: {}", e.message()));
            return (config, problems);
        }
    };
    for (key, value) in &table {
        match key.as_str() {
            "example_chapter" => match value.as_bool() {
                Some(on) => config.example_chapter = on,
                None => problems.push(format!("`example_chapter` must be true or false, not {value}")),
            },
            other => problems.push(format!("unknown setting `{other}`, ignored")),
        }
    }
    (config, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_text_reads_as_the_defaults() {
        assert_eq!(parse(DEFAULT_TEXT), (ServerConfig::default(), Vec::new()));
        assert_eq!(parse(""), (ServerConfig::default(), Vec::new()));
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
