//! Save data for a session. Everyone plays with the host's save (same unlocked levels on every
//! PC). Every time the game saves, every PC writes all of the session's saves to its co-op save
//! for this game (`<save dir>/coop/<game file>.save`), never to the solo save. The host starts
//! from its co-op save, or from its solo save the first time.

use ruffle_core::backend::storage::StorageBackend;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A save's name without the folder the game file is in ("localhost/<folder>/game.swf/so"
/// becomes "game.swf/so"), so a friend who keeps the game somewhere else finds the host's save.
fn key(name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    match parts.iter().rposition(|part| part.to_ascii_lowercase().ends_with(".swf")) {
        Some(at) => parts[at..].join("/"),
        None => name.to_string(),
    }
}

/// This PC's co-op save for a game: one file holding all its saves, packed like `to_bytes`.
pub fn coop_save_file(save_dir: &Path, swf_file_name: &str) -> PathBuf {
    save_dir.join("coop").join(format!("{swf_file_name}.save"))
}

#[derive(Default)]
pub struct SessionStorage {
    data: BTreeMap<String, Vec<u8>>,
    /// The co-op save file; None writes nothing (tests, replays).
    file: Option<PathBuf>,
    /// Set once the games are out of sync: a broken game never overwrites a good save.
    frozen: Arc<AtomicBool>,
}

/// Host: which save the session starts from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum SaveStart {
    /// The host's co-op save for this game, or its solo save if it has none yet.
    #[default]
    Coop,
    /// The host's solo save, even if it has a co-op save.
    Solo,
    /// No save: the game starts as if new.
    New,
}

/// Stops a session's co-op save from changing (once the games are out of sync).
#[derive(Clone)]
pub struct SaveGate(Arc<AtomicBool>);

impl SaveGate {
    pub fn freeze(&self) {
        if !self.0.swap(true, Ordering::Relaxed) {
            tracing::warn!("Ruffle Together: out of sync, so this game is no longer saved");
        }
    }
}

impl SessionStorage {
    /// Also keeps everything the game saves in `file` (see `coop_save_file`).
    pub fn with_coop_save(mut self, file: PathBuf) -> (Self, SaveGate) {
        self.file = Some(file);
        let gate = SaveGate(self.frozen.clone());
        (self, gate)
    }

    /// The host's saves for this game, as `start` picks them (see `SaveStart`). With Solo or New
    /// the co-op save is replaced the first time the game saves.
    pub fn export_for_session(save_dir: &Path, swf_file_name: &str, start: SaveStart) -> Vec<u8> {
        match start {
            SaveStart::New => return vec![],
            SaveStart::Solo => return Self::export_from_disk(save_dir, swf_file_name),
            SaveStart::Coop => {}
        }
        match fs::read(coop_save_file(save_dir, swf_file_name)) {
            Ok(bytes) if Self::from_bytes(&bytes).is_some() => bytes,
            Ok(_) => {
                tracing::warn!("Ruffle Together: the co-op save of {swf_file_name} is damaged; using the solo save");
                Self::export_from_disk(save_dir, swf_file_name)
            }
            Err(_) => Self::export_from_disk(save_dir, swf_file_name),
        }
    }

    /// Writes every save to the co-op save file: beside it first, then in its place, so a crash
    /// mid-write keeps the old one.
    fn write_coop_save(&self) {
        let Some(file) = &self.file else { return };
        if self.frozen.load(Ordering::Relaxed) {
            return;
        }
        let temp = file.with_extension("save.tmp");
        let result = file
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&temp, self.to_bytes()))
            .and_then(|()| fs::rename(&temp, file));
        if let Err(e) = result {
            tracing::warn!("Ruffle Together: couldn't save {}: {e}", file.display());
        }
    }

    /// Unpacks `to_bytes` output; None if it is cut short or malformed.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        fn take<'a>(bytes: &mut &'a [u8]) -> Option<&'a [u8]> {
            let len = u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
            let chunk = bytes.get(4..4 + len)?;
            *bytes = &bytes[4 + len..];
            Some(chunk)
        }
        let mut rest = bytes;
        let mut data = BTreeMap::new();
        while !rest.is_empty() {
            let name = String::from_utf8(take(&mut rest)?.to_vec()).ok()?;
            let value = take(&mut rest)?.to_vec();
            data.insert(key(&name), value);
        }
        Some(Self { data, ..Self::default() })
    }

    /// Each entry as [name length, name, value length, value], lengths as big-endian u32.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![];
        for (name, value) in &self.data {
            for chunk in [name.as_bytes(), value.as_slice()] {
                out.extend_from_slice(&(chunk.len() as u32).to_be_bytes());
                out.extend_from_slice(chunk);
            }
        }
        out
    }

    /// The host's saves for this game from Ruffle's save folder (`<name>.sol` files whose path
    /// contains the game's file name), packed like `to_bytes`. Other games' saves stay private.
    pub fn export_from_disk(save_dir: &Path, swf_file_name: &str) -> Vec<u8> {
        fn walk(dir: &Path, root: &Path, swf: &str, out: &mut SessionStorage) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, swf, out);
                } else if path.extension().is_some_and(|e| e == "sol")
                    && let Ok(rel) = path.with_extension("").strip_prefix(root)
                {
                    let name = rel.to_string_lossy().replace('\\', "/");
                    if name.split('/').any(|part| part == swf)
                        && let Ok(value) = fs::read(&path)
                    {
                        out.data.insert(key(&name), value);
                    }
                }
            }
        }
        let mut storage = SessionStorage::default();
        walk(save_dir, save_dir, swf_file_name, &mut storage);
        storage.to_bytes()
    }
}

impl StorageBackend for SessionStorage {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.data.get(&key(name)).cloned()
    }

    fn put(&mut self, name: &str, value: &[u8]) -> bool {
        self.data.insert(key(name), value.to_vec());
        self.write_coop_save();
        true
    }

    fn remove_key(&mut self, name: &str) {
        self.data.remove(&key(name));
        self.write_coop_save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bytes() {
        let mut storage = SessionStorage::default();
        storage.put("localhost/game.swf/so_save", &[1, 2, 3]);
        storage.put("localhost/game.swf/other", &[]);
        let copy = SessionStorage::from_bytes(&storage.to_bytes()).expect("valid bytes");
        assert_eq!(copy.get("localhost/game.swf/so_save"), Some(vec![1, 2, 3]));
        assert_eq!(copy.get("localhost/game.swf/other"), Some(vec![]));
        assert_eq!(copy.get("missing"), None);
    }

    #[test]
    fn bad_bytes_are_refused() {
        assert!(SessionStorage::from_bytes(&[0, 0, 0, 9, 1]).is_none());
    }

    #[test]
    fn friend_in_another_folder_finds_the_hosts_save() {
        let mut host = SessionStorage::default();
        host.put("localhost/Users/liam/games/BadIceCream_coop.swf/so_badicecream", &[5]);
        let friend = SessionStorage::from_bytes(&host.to_bytes()).expect("valid bytes");
        assert_eq!(
            friend.get("localhost/Downloads/BadIceCream_coop.swf/so_badicecream"),
            Some(vec![5])
        );
    }

    #[test]
    fn put_does_not_touch_disk() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut storage = SessionStorage::default();
        storage.put("so_badicecream", &[9]);
        storage.remove_key("so_badicecream");
        assert_eq!(fs::read_dir(dir.path()).expect("read dir").count(), 0);
    }

    #[test]
    fn what_the_game_saves_goes_to_the_coop_save_only() {
        let dir = tempfile::tempdir().expect("temp dir");
        let solo = dir.path().join("localhost").join("Game.swf");
        fs::create_dir_all(&solo).expect("mkdir");
        fs::write(solo.join("so.sol"), [1]).expect("write");
        let bytes = SessionStorage::export_for_session(dir.path(), "Game.swf", SaveStart::Coop);
        let (mut storage, _gate) = SessionStorage::from_bytes(&bytes)
            .expect("valid bytes")
            .with_coop_save(coop_save_file(dir.path(), "Game.swf"));
        assert_eq!(storage.get("localhost/x/Game.swf/so"), Some(vec![1]), "starts from the solo save");
        storage.put("localhost/x/Game.swf/so", &[2]);
        // a save with no game file in its name (a SharedObject with localPath "/")
        storage.put("localhost//top", &[3]);
        assert_eq!(fs::read(solo.join("so.sol")).expect("solo save"), [1], "the solo save is untouched");

        let next = SessionStorage::from_bytes(&SessionStorage::export_for_session(dir.path(), "Game.swf", SaveStart::Coop))
            .expect("valid bytes");
        assert_eq!(next.get("localhost/y/Game.swf/so"), Some(vec![2]), "the next session continues");
        assert_eq!(next.get("localhost//top"), Some(vec![3]));
        assert!(SessionStorage::export_for_session(dir.path(), "Other.swf", SaveStart::Coop).is_empty());
    }

    #[test]
    fn the_host_can_start_from_its_solo_save_or_from_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let solo = dir.path().join("localhost").join("Game.swf");
        fs::create_dir_all(&solo).expect("mkdir");
        fs::write(solo.join("so.sol"), [1]).expect("write");
        let (mut coop, _gate) = SessionStorage::default().with_coop_save(coop_save_file(dir.path(), "Game.swf"));
        coop.put("Game.swf/so", &[2]);
        let start = |start| {
            SessionStorage::from_bytes(&SessionStorage::export_for_session(dir.path(), "Game.swf", start))
                .expect("valid bytes")
                .get("Game.swf/so")
        };
        assert_eq!(start(SaveStart::Coop), Some(vec![2]));
        assert_eq!(start(SaveStart::Solo), Some(vec![1]));
        assert_eq!(start(SaveStart::New), None);
    }

    #[test]
    fn an_out_of_sync_game_is_not_saved() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = coop_save_file(dir.path(), "Game.swf");
        let (mut storage, gate) = SessionStorage::default().with_coop_save(file.clone());
        storage.put("Game.swf/so", &[1]);
        gate.freeze();
        storage.put("Game.swf/so", &[2]);
        storage.remove_key("Game.swf/so");
        let saved = SessionStorage::from_bytes(&fs::read(&file).expect("co-op save")).expect("valid");
        assert_eq!(saved.get("Game.swf/so"), Some(vec![1]));
        assert_eq!(storage.get("Game.swf/so"), None, "the game itself still sees its own writes");
    }

    #[test]
    fn a_damaged_coop_save_falls_back_to_the_solo_save() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = coop_save_file(dir.path(), "Game.swf");
        fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
        fs::write(&file, [0, 0, 0, 9]).expect("write");
        let solo = dir.path().join("localhost").join("Game.swf");
        fs::create_dir_all(&solo).expect("mkdir");
        fs::write(solo.join("so.sol"), [4]).expect("write");
        let storage = SessionStorage::from_bytes(&SessionStorage::export_for_session(dir.path(), "Game.swf", SaveStart::Coop))
            .expect("valid bytes");
        assert_eq!(storage.get("Game.swf/so"), Some(vec![4]));
    }

    #[test]
    fn export_takes_only_this_games_saves() {
        let dir = tempfile::tempdir().expect("temp dir");
        let game = dir.path().join("localhost").join("BadIceCream_coop.swf");
        fs::create_dir_all(&game).expect("mkdir");
        fs::write(game.join("so_badicecream.sol"), [7, 7]).expect("write");
        let other = dir.path().join("localhost").join("Other.swf");
        fs::create_dir_all(&other).expect("mkdir");
        fs::write(other.join("secret.sol"), [1]).expect("write");

        let storage = SessionStorage::from_bytes(&SessionStorage::export_from_disk(
            dir.path(),
            "BadIceCream_coop.swf",
        ))
        .expect("valid bytes");
        assert_eq!(
            storage.get("localhost/BadIceCream_coop.swf/so_badicecream"),
            Some(vec![7, 7])
        );
        assert_eq!(storage.get("localhost/Other.swf/secret"), None);
    }
}
