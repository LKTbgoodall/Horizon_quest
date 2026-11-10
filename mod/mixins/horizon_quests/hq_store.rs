// The mod's files in the world folder (`<world>/horizon-quests/`): JSON,
// written whole and atomically. A missing file is a fresh start; a file that
// cannot be read is an error and is never overwritten, so a player's
// progress is not lost to a bad save or a newer version of the mod. Pure: no
// game types.
use super::hq_model::{FORMAT_VERSION, Progress};
use super::hq_teams::Teams;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The mod's folder in a world's folder.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub const FOLDER: &str = "horizon-quests";

/// The value in the JSON file at `path`, or its default when there is no
/// file. Err when the file is there but is not that.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text))
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Writes `value` as the JSON file at `path`, making its folder.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("{}: {e}", path.display()))?;
    write_atomic(path, text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))
}

/// Replaces the file at `path` in one step: a crash leaves the old file or
/// the new one, never half of one.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    let mut file = std::fs::File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temp, path)
}

/// Where team `team`'s progress is kept under the mod's folder `dir`. None
/// when the team's id cannot be a file name.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn progress_path(dir: &Path, team: &str) -> Option<PathBuf> {
    let safe = (1..=64).contains(&team.len()) && team.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    safe.then(|| dir.join("progress").join(format!("{team}.json")))
}

/// Team `team`'s progress: empty when it has no file yet.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn load_progress(dir: &Path, team: &str) -> Result<Progress, String> {
    let path = progress_path(dir, team).ok_or_else(|| format!("`{team}` cannot name a progress file"))?;
    let mut progress: Progress = read_json(&path)?;
    if progress.version > FORMAT_VERSION {
        return Err(format!(
            "{}: written by a newer Horizon Quests (version {}, this one reads {FORMAT_VERSION})",
            path.display(),
            progress.version
        ));
    }
    // Version 1 is the first: nothing to bring up to date yet.
    progress.version = FORMAT_VERSION;
    Ok(progress)
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn save_progress(dir: &Path, team: &str, progress: &Progress) -> Result<(), String> {
    let path = progress_path(dir, team).ok_or_else(|| format!("`{team}` cannot name a progress file"))?;
    save_json(&path, progress)
}

/// The world's parties (hq_teams): none when there is no file yet.
pub fn load_teams(dir: &Path) -> Result<Teams, String> {
    let path = dir.join(TEAMS_FILE);
    let mut teams: Teams = read_json(&path)?;
    if teams.version > FORMAT_VERSION {
        return Err(format!(
            "{}: written by a newer Horizon Quests (version {}, this one reads {FORMAT_VERSION})",
            path.display(),
            teams.version
        ));
    }
    teams.version = FORMAT_VERSION;
    Ok(teams)
}

pub fn save_teams(dir: &Path, teams: &Teams) -> Result<(), String> {
    save_json(&dir.join(TEAMS_FILE), teams)
}

/// The parties' file in the mod's folder.
pub const TEAMS_FILE: &str = "teams.json";

/// A player's UUID as files and progress name it:
/// `069a79f4-44e9-4726-a5be-fca90e38aaf5`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn uuid_string(uuid: u128) -> String {
    let hex = format!("{uuid:032x}");
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// The UUID `text` writes, with or without dashes.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn parse_uuid(text: &str) -> Option<u128> {
    let hex: String = text.chars().filter(|&c| c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty folder of this test's own.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hq-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn progress_round_trips_through_its_file() {
        let dir = folder("round-trip");
        let team = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        assert_eq!(load_progress(&dir, team), Ok(Progress::default()));

        let mut progress = Progress::default();
        progress.tasks.insert("a/q/t".into(), 3);
        progress.completed.insert("a/done".into(), 20);
        progress.claimed.entry("a/done/r".into()).or_default().insert(team.into());
        save_progress(&dir, team, &progress).unwrap();
        assert_eq!(load_progress(&dir, team), Ok(progress.clone()));

        // Saving again replaces the file and leaves nothing else behind.
        progress.tasks.insert("a/q/t".into(), 4);
        save_progress(&dir, team, &progress).unwrap();
        assert_eq!(load_progress(&dir, team), Ok(progress));
        let files: Vec<_> = std::fs::read_dir(dir.join("progress")).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(files, [std::ffi::OsString::from(format!("{team}.json"))]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_that_cannot_be_read_is_an_error_and_stays() {
        let dir = folder("broken");
        let path = progress_path(&dir, "steve").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        std::fs::write(&path, "{ half a file").unwrap();
        assert!(load_progress(&dir, "steve").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ half a file");

        std::fs::write(&path, r#"{ "version": 99 }"#).unwrap();
        let err = load_progress(&dir, "steve").unwrap_err();
        assert!(err.contains("newer Horizon Quests (version 99"), "{err}");

        // An older or unmarked file loads as the current version.
        std::fs::write(&path, "\u{feff}{ \"version\": 0, \"tasks\": { \"a/q/t\": 2 } }").unwrap();
        let progress = load_progress(&dir, "steve").unwrap();
        assert_eq!((progress.version, progress.task("a/q/t")), (FORMAT_VERSION, 2));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn team_ids_must_be_file_names() {
        let dir = Path::new("world");
        assert_eq!(progress_path(dir, "team_1"), Some(dir.join("progress").join("team_1.json")));
        for bad in ["", "../steve", "a/b", "a\\b", "a.b", "con:", &"x".repeat(65)] {
            assert_eq!(progress_path(dir, bad), None, "{bad}");
            assert!(load_progress(dir, bad).is_err() && save_progress(dir, bad, &Progress::default()).is_err());
        }
    }

    #[test]
    fn teams_round_trip_and_a_newer_file_is_refused() {
        let dir = folder("teams");
        assert_eq!(load_teams(&dir), Ok(Teams::default()));
        let mut teams = Teams::default();
        teams.create("steve", "Steve", "Miners", 1, &mut || 3).unwrap();
        save_teams(&dir, &teams).unwrap();
        assert_eq!(load_teams(&dir), Ok(teams));
        std::fs::write(dir.join(TEAMS_FILE), r#"{ "version": 99 }"#).unwrap();
        assert!(load_teams(&dir).unwrap_err().contains("newer Horizon Quests"));
        std::fs::write(dir.join(TEAMS_FILE), "{ broken").unwrap();
        assert!(load_teams(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn uuids() {
        let uuid = 0x069a79f4_44e9_4726_a5be_fca90e38aaf5_u128;
        assert_eq!(uuid_string(uuid), "069a79f4-44e9-4726-a5be-fca90e38aaf5");
        assert_eq!(uuid_string(1), "00000000-0000-0000-0000-000000000001");
        assert_eq!(parse_uuid("069a79f4-44e9-4726-a5be-fca90e38aaf5"), Some(uuid));
        assert_eq!(parse_uuid("069A79F444E94726A5BEFCA90E38AAF5"), Some(uuid));
        assert_eq!(parse_uuid(&uuid_string(u128::MAX)), Some(u128::MAX));
        for bad in ["", "steve", "069a79f4-44e9-4726-a5be-fca90e38aaf", "+69a79f444e94726a5befca90e38aaf5"] {
            assert_eq!(parse_uuid(bad), None, "{bad}");
        }
        assert!(progress_path(Path::new("w"), &uuid_string(uuid)).is_some());
    }
}
