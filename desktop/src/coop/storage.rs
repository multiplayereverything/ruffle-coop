//! Save data for a session. Everyone plays with the host's save (same unlocked levels on every
//! PC); writes stay in memory, so nobody's real save changes.

use ruffle_core::backend::storage::StorageBackend;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// A save's name without the folder the game file is in ("localhost/<folder>/game.swf/so"
/// becomes "game.swf/so"), so a friend who keeps the game somewhere else finds the host's save.
fn key(name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    match parts.iter().rposition(|part| part.to_ascii_lowercase().ends_with(".swf")) {
        Some(at) => parts[at..].join("/"),
        None => name.to_string(),
    }
}

#[derive(Default)]
pub struct SessionStorage {
    data: BTreeMap<String, Vec<u8>>,
}

impl SessionStorage {
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
        Some(Self { data })
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
        true
    }

    fn remove_key(&mut self, name: &str) {
        self.data.remove(&key(name));
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
