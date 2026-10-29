use chrono::Utc;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default)]
pub enum FilenamePattern {
    #[default]
    SingleFile,
    WithTimestamp,
}

impl FromStr for FilenamePattern {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "single_file" => Ok(FilenamePattern::SingleFile),
            "with_timestamp" => Ok(FilenamePattern::WithTimestamp),
            _ => Err(()),
        }
    }
}

impl FilenamePattern {
    pub fn create_path(self, directory: &Path) -> PathBuf {
        match self {
            FilenamePattern::SingleFile => directory.join("ruffle.log"),
            FilenamePattern::WithTimestamp => {
                directory.join(Utc::now().format("ruffle_%F_%H-%M-%S.log").to_string())
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FilenamePattern::SingleFile => "single_file",
            FilenamePattern::WithTimestamp => "with_timestamp",
        }
    }
}

/// Where this run logs. Headless sessions (labs and tests, often several at once) write
/// `ruffle-headless.log`, so they never push a player's own game logs out.
pub fn log_path(pattern: FilenamePattern, directory: &Path, headless: bool) -> PathBuf {
    if headless {
        directory.join("ruffle-headless.log")
    } else {
        pattern.create_path(directory)
    }
}

/// How many logs of earlier runs stay next to `ruffle.log` (as `ruffle.1.log` = the last run,
/// `ruffle.2.log`, ...): a game that stopped can still be looked into after Ruffle starts again.
pub const OLD_LOGS: usize = 5;

/// Moves the log at `path` and its older copies one place down before a new log is written
/// there; the oldest goes. Missing files are fine.
pub fn keep_old_logs(path: &Path, keep: usize) {
    let old = |n: usize| path.with_extension(format!("{n}.log"));
    let _ = std::fs::remove_file(old(keep));
    for n in (1..keep).rev() {
        let _ = std::fs::rename(old(n), old(n + 1));
    }
    let _ = std::fs::rename(path, old(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_logs_move_down_and_the_oldest_goes() {
        let dir = std::env::temp_dir().join(format!("ruffle-logs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let log = dir.join("ruffle.log");
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).ok();
        keep_old_logs(&log, 3); // nothing there yet
        for run in 1..=5 {
            keep_old_logs(&log, 3);
            std::fs::write(&log, format!("run {run}")).expect("write");
        }
        assert_eq!(read("ruffle.log").as_deref(), Some("run 5"));
        assert_eq!(read("ruffle.1.log").as_deref(), Some("run 4"));
        assert_eq!(read("ruffle.2.log").as_deref(), Some("run 3"));
        assert_eq!(read("ruffle.3.log").as_deref(), Some("run 2"));
        assert_eq!(read("ruffle.4.log"), None, "only 3 old ones");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Lab runs started six headless players at once and pushed a real game's log out.
    #[test]
    fn headless_runs_keep_their_own_log() {
        let dir = Path::new("logs");
        assert_eq!(log_path(FilenamePattern::SingleFile, dir, true), dir.join("ruffle-headless.log"));
        assert_eq!(log_path(FilenamePattern::SingleFile, dir, false), dir.join("ruffle.log"));
    }
}
