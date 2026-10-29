//! Keys in and out. A key map says, for each player slot, which game keys the slot's buttons press
//! (as Flash key codes), and which keys a player presses on their own PC for each button. Maps are
//! small text files: built in for the games Ruffle Together ships with, or any file
//! (`--coop-keys <file>`), so a game can bring its own keys without a new build.
//!
//! ```text
//! # comment
//! shared                                    every slot presses the same keys (a hot-seat game)
//! slot 0: UP=38 DOWN=40 LEFT=37 RIGHT=39 FIRE=90 ALT=88
//! slot 1: UP=87 DOWN=83 LEFT=65 RIGHT=68 FIRE=84 ALT=89
//! local: UP=38,87 DOWN=40,83 FIRE=90,84     keys a player presses on their own PC, per button
//! ```

use crate::coop::lockstep::{MAX_PLAYERS, button_named};
use ruffle_core::PlayerEvent;
use ruffle_core::events::{KeyDescriptor, KeyLocation, LogicalKey, NamedKey, PhysicalKey};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// The key maps built into this build.
const BUILT_IN: &[(&str, &str)] = &[
    ("bad-ice-cream", include_str!("keys/bad-ice-cream.keys")),
    ("gun-mayhem-2", include_str!("keys/gun-mayhem-2.keys")),
    ("territory-war", include_str!("keys/territory-war.keys")),
    ("bow-man", include_str!("keys/bow-man.keys")),
    ("smileys-war", include_str!("keys/smileys-war.keys")),
    ("stealth-assassin", include_str!("keys/stealth-assassin.keys")),
    ("stealth-hunter-2", include_str!("keys/stealth-hunter-2.keys")),
    ("intrusion", include_str!("keys/intrusion.keys")),
    ("intrusion-2", include_str!("keys/intrusion-2.keys")),
    ("air-battle", include_str!("keys/air-battle.keys")),
    ("ultimate-crab-battle", include_str!("keys/ultimate-crab-battle.keys")),
];

/// A game's key map (`--coop-keys`).
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    name: String,
    /// What every PC compares in the handshake: the name and a hash of the map's text.
    id: String,
    shared: bool,
    /// Per slot: (button, key code), in the file's order.
    slots: Vec<Vec<(u16, u32)>>,
    /// (key code a player presses, button).
    local: Vec<(u32, u16)>,
}

impl Game {
    pub fn built_in_names() -> Vec<&'static str> {
        BUILT_IN.iter().map(|(name, _)| *name).collect()
    }

    pub fn built_in(name: &str) -> Option<Self> {
        let (name, text) = BUILT_IN.iter().find(|(n, _)| *n == name)?;
        Some(Self::parse(name, text).expect("built-in key maps parse"))
    }

    /// A built-in name, or the path of a key map file.
    pub fn load(spec: &str) -> Result<Self, String> {
        if let Some(game) = Self::built_in(spec) {
            return Ok(game);
        }
        let path = std::path::Path::new(spec);
        let text = std::fs::read_to_string(path).map_err(|_| {
            format!("no keys for \"{spec}\": not a file, and not one of {}", Self::built_in_names().join(", "))
        })?;
        let name = path.file_stem().map_or(spec.into(), |s| s.to_string_lossy().into_owned());
        Self::parse(&name, &text).map_err(|e| format!("{spec}: {e}"))
    }

    pub fn parse(name: &str, text: &str) -> Result<Self, String> {
        let mut game = Game {
            name: name.to_string(),
            id: format!("{name}#{:.12x}", Sha256::digest(text.replace('\r', "").as_bytes())),
            shared: false,
            slots: vec![],
            local: vec![],
        };
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            let fail = |why: String| format!("line {}: {why}", number + 1);
            let pairs = |list: &str| -> Result<Vec<(u16, Vec<u32>)>, String> {
                list.split_whitespace()
                    .map(|pair| {
                        let (button, codes) = pair.split_once('=').ok_or_else(|| fail(format!("expected BUTTON=code: {pair}")))?;
                        let button = button_named(button).ok_or_else(|| fail(format!("unknown button {button}")))?;
                        let codes = codes
                            .split(',')
                            .map(|c| {
                                c.parse::<u32>()
                                    .ok()
                                    .filter(|&c| key_descriptor(c).is_some())
                                    .ok_or_else(|| fail(format!("no key with code {c}")))
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok((button, codes))
                    })
                    .collect()
            };
            if line.is_empty() {
                continue;
            } else if line == "shared" {
                game.shared = true;
            } else if let Some(list) = line.strip_prefix("local:") {
                for (button, codes) in pairs(list)? {
                    game.local.extend(codes.into_iter().map(|code| (code, button)));
                }
            } else if let Some(rest) = line.strip_prefix("slot ") {
                let (slot, list) = rest.split_once(':').ok_or_else(|| fail("expected slot N: ...".into()))?;
                if slot.trim().parse::<usize>().ok() != Some(game.slots.len()) || game.slots.len() >= MAX_PLAYERS {
                    return Err(fail(format!("slots go 0, 1, 2, 3 in order (got {slot})")));
                }
                let keys = pairs(list)?
                    .into_iter()
                    .map(|(button, codes)| match codes[..] {
                        [code] => Ok((button, code)),
                        _ => Err(fail("a slot button presses one key".into())),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                game.slots.push(keys);
            } else {
                return Err(fail(format!("unknown line: {line}")));
            }
        }
        if game.slots.is_empty() {
            return Err("no slots".into());
        }
        Ok(game)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name and a hash of the map: every player must have the same.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// How many players the game has keys for.
    pub fn max_players(&self) -> u8 {
        self.slots.len() as u8
    }

    /// The keys each player slot presses in the game.
    pub fn keys(&self) -> GameKeys {
        let slots: Vec<Vec<GameKey>> = self
            .slots
            .iter()
            .map(|slot| {
                slot.iter()
                    .map(|&(button, code)| GameKey { button, key: key_descriptor(code).expect("checked when parsed") })
                    .collect()
            })
            .collect();
        GameKeys {
            held: vec![0; slots.len()],
            slots,
            shared: self.shared,
        }
    }

    /// This PC's buttons from the keys held right now.
    pub fn local_buttons(&self, held: &HashSet<PhysicalKey>) -> u16 {
        let mut buttons = 0;
        for &key in held {
            let pressed = KeyDescriptor { physical_key: key, logical_key: LogicalKey::Unknown, key_location: KeyLocation::Standard };
            if let (Some(code), _) = ruffle_core::coop_key_codes(pressed) {
                buttons |= self.local.iter().filter(|(c, _)| *c == code).fold(0, |all, (_, b)| all | b);
            }
        }
        buttons
    }
}

/// The key a game sees for a Flash key code: the physical key and the logical key both map to it.
fn key_descriptor(code: u32) -> Option<KeyDescriptor> {
    use PhysicalKey as P;
    const LETTERS: [PhysicalKey; 26] = [
        P::KeyA, P::KeyB, P::KeyC, P::KeyD, P::KeyE, P::KeyF, P::KeyG, P::KeyH, P::KeyI, P::KeyJ, P::KeyK, P::KeyL, P::KeyM,
        P::KeyN, P::KeyO, P::KeyP, P::KeyQ, P::KeyR, P::KeyS, P::KeyT, P::KeyU, P::KeyV, P::KeyW, P::KeyX, P::KeyY, P::KeyZ,
    ];
    const DIGITS: [PhysicalKey; 10] =
        [P::Digit0, P::Digit1, P::Digit2, P::Digit3, P::Digit4, P::Digit5, P::Digit6, P::Digit7, P::Digit8, P::Digit9];
    const NUMPAD: [PhysicalKey; 10] =
        [P::Numpad0, P::Numpad1, P::Numpad2, P::Numpad3, P::Numpad4, P::Numpad5, P::Numpad6, P::Numpad7, P::Numpad8, P::Numpad9];
    let character = |physical_key, ch| KeyDescriptor { physical_key, logical_key: LogicalKey::Character(ch), key_location: KeyLocation::Standard };
    let numpad = |physical_key, ch| KeyDescriptor { physical_key, logical_key: LogicalKey::Character(ch), key_location: KeyLocation::Numpad };
    let named = |physical_key, named| KeyDescriptor { physical_key, logical_key: LogicalKey::Named(named), key_location: KeyLocation::Standard };
    let digit = |n: u32| char::from_digit(n, 10).unwrap_or('0');
    Some(match code {
        65..=90 => character(LETTERS[(code - 65) as usize], (b'a' + (code - 65) as u8) as char),
        48..=57 => character(DIGITS[(code - 48) as usize], digit(code - 48)),
        96..=105 => numpad(NUMPAD[(code - 96) as usize], digit(code - 96)),
        106 => numpad(P::NumpadMultiply, '*'),
        107 => numpad(P::NumpadAdd, '+'),
        109 => numpad(P::NumpadSubtract, '-'),
        110 => numpad(P::NumpadDecimal, '.'),
        111 => numpad(P::NumpadDivide, '/'),
        32 => character(P::Space, ' '),
        37 => named(P::ArrowLeft, NamedKey::ArrowLeft),
        38 => named(P::ArrowUp, NamedKey::ArrowUp),
        39 => named(P::ArrowRight, NamedKey::ArrowRight),
        40 => named(P::ArrowDown, NamedKey::ArrowDown),
        8 => named(P::Backspace, NamedKey::Backspace),
        9 => named(P::Tab, NamedKey::Tab),
        13 => named(P::Enter, NamedKey::Enter),
        16 => named(P::ShiftLeft, NamedKey::Shift),
        17 => named(P::ControlLeft, NamedKey::Control),
        18 => named(P::AltLeft, NamedKey::Alt),
        27 => named(P::Escape, NamedKey::Escape),
        33 => named(P::PageUp, NamedKey::PageUp),
        34 => named(P::PageDown, NamedKey::PageDown),
        35 => named(P::End, NamedKey::End),
        36 => named(P::Home, NamedKey::Home),
        45 => named(P::Insert, NamedKey::Insert),
        46 => named(P::Delete, NamedKey::Delete),
        186 => character(P::Semicolon, ';'),
        187 => character(P::Equal, '='),
        188 => character(P::Comma, ','),
        189 => character(P::Minus, '-'),
        190 => character(P::Period, '.'),
        191 => character(P::Slash, '/'),
        192 => character(P::Backquote, '`'),
        219 => character(P::BracketLeft, '['),
        220 => character(P::Backslash, '\\'),
        221 => character(P::BracketRight, ']'),
        222 => character(P::Quote, '\''),
        _ => return None,
    })
}

#[derive(Clone, Copy)]
struct GameKey {
    button: u16,
    key: KeyDescriptor,
}

/// The game's own keys for each player slot, and what each slot holds now.
pub struct GameKeys {
    slots: Vec<Vec<GameKey>>,
    held: Vec<u16>,
    /// Every slot presses the same keys (a hot-seat game): a key is down while any player holds it.
    shared: bool,
}

impl GameKeys {
    /// Key presses and releases that turn `slot`'s held buttons into `buttons`.
    pub fn events(&mut self, slot: u8, buttons: u16) -> Vec<PlayerEvent> {
        let Some(keys) = self.slots.get(slot as usize) else {
            return vec![];
        };
        let union = |held: &[u16]| held.iter().fold(0, |all, b| all | b);
        let before_all = union(&self.held);
        let before = std::mem::replace(&mut self.held[slot as usize], buttons);
        // shared keys: what changes is the OR of every player's buttons
        let (before, buttons) = if self.shared {
            (before_all, union(&self.held))
        } else {
            (before, buttons)
        };
        let mut events = vec![];
        for game_key in keys {
            let (was, is) = (before & game_key.button != 0, buttons & game_key.button != 0);
            if is && !was {
                events.push(PlayerEvent::KeyDown { key: game_key.key });
                // a real keyboard also types the character: on(keyPress "1") and text fields hear it
                if let LogicalKey::Character(codepoint) = game_key.key.logical_key {
                    events.push(PlayerEvent::TextInput { codepoint });
                }
            } else if was && !is {
                events.push(PlayerEvent::KeyUp { key: game_key.key });
            }
        }
        events
    }

    /// Releases everything held, for every slot.
    pub fn release_all(&mut self) -> Vec<PlayerEvent> {
        (0..self.slots.len() as u8)
            .flat_map(|slot| self.events(slot, 0))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coop::lockstep::{ALT, DOWN, FIRE, KEY1, KEY2, KEY3, KEY4, LEFT, NO, QUIT, RIGHT, UP, VIEW, YES};

    fn game(name: &str) -> Game {
        Game::built_in(name).expect("built in")
    }

    fn held(keys: &[PhysicalKey]) -> HashSet<PhysicalKey> {
        keys.iter().copied().collect()
    }

    fn down_keys(events: &[PlayerEvent]) -> Vec<PhysicalKey> {
        events
            .iter()
            .filter_map(|e| match e {
                PlayerEvent::KeyDown { key } => Some(key.physical_key),
                _ => None,
            })
            .collect()
    }

    fn up_keys(events: &[PlayerEvent]) -> Vec<PhysicalKey> {
        events
            .iter()
            .filter_map(|e| match e {
                PlayerEvent::KeyUp { key } => Some(key.physical_key),
                _ => None,
            })
            .collect()
    }

    /// Every key code a map can name reaches the game as that code, by Ruffle's physical mapping
    /// and by its logical one (the default), and every built-in map parses.
    #[test]
    fn key_codes_round_trip_and_built_in_maps_parse() {
        let mut known = 0;
        for code in 0..=255 {
            if let Some(key) = key_descriptor(code) {
                assert_eq!(ruffle_core::coop_key_codes(key), (Some(code), Some(code)), "code {code}");
                known += 1;
            }
        }
        assert_eq!(known, 80, "letters, digits, numpad, arrows, named keys and punctuation");
        for name in Game::built_in_names() {
            assert!(Game::built_in(name).is_some(), "{name}");
        }
    }

    #[test]
    fn bad_key_map_files_say_what_is_wrong() {
        let err = |text: &str| Game::parse("t", text).err().expect("an error");
        assert!(err("slot 0: UP=38 JUMP=32").contains("unknown button JUMP"));
        assert!(err("slot 0: UP=999").contains("no key with code 999"));
        assert!(err("slot 1: UP=38").contains("slots go 0, 1, 2, 3"));
        assert!(err("slot 0: UP=38,87").contains("one key"));
        assert!(err("players 2\nslot 0: UP=38").contains("line 1: unknown line"));
        assert_eq!(err("# nothing"), "no slots");
        let a = Game::parse("t", "slot 0: UP=38").expect("parses");
        let b = Game::parse("t", "slot 0: UP=40").expect("parses");
        assert_ne!(a.id(), b.id(), "a different map is refused in the handshake");
    }

    #[test]
    fn a_key_map_file_can_be_loaded() {
        let dir = std::env::temp_dir().join(format!("coop-keys-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("my-game.keys");
        std::fs::write(&path, "slot 0: FIRE=32\r\nslot 1: FIRE=13\r\nlocal: FIRE=32\r\n").expect("write");
        let game = Game::load(&path.to_string_lossy()).expect("loads");
        assert_eq!((game.name(), game.max_players()), ("my-game", 2));
        assert_eq!(game.local_buttons(&held(&[PhysicalKey::Space])), FIRE);
        assert!(Game::load("pong").err().expect("an error").contains("gun-mayhem-2"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn wasd_and_arrows_both_move() {
        let bic = game("bad-ice-cream");
        assert_eq!(bic.local_buttons(&held(&[PhysicalKey::KeyW])), UP);
        assert_eq!(bic.local_buttons(&held(&[PhysicalKey::ArrowUp])), UP);
        assert_eq!(bic.local_buttons(&held(&[PhysicalKey::KeyA, PhysicalKey::ArrowDown])), LEFT | DOWN);
        assert_eq!(bic.local_buttons(&held(&[PhysicalKey::KeyD])), RIGHT);
        for fire in [PhysicalKey::Space, PhysicalKey::Enter, PhysicalKey::KeyF] {
            assert_eq!(bic.local_buttons(&held(&[fire])), FIRE);
        }
        assert_eq!(bic.local_buttons(&held(&[PhysicalKey::KeyQ])), 0);
    }

    /// Gun Mayhem 2's own defaults (savedata2.controlarray): player 1 arrows + Z shoot + X
    /// grenade, player 2 W A S D + T shoot + Y grenade; a player may use either set on their PC.
    #[test]
    fn gun_mayhem_2_keys() {
        let gm2 = game("gun-mayhem-2");
        assert_eq!(gm2.local_buttons(&held(&[PhysicalKey::ArrowLeft])), LEFT);
        assert_eq!(gm2.local_buttons(&held(&[PhysicalKey::KeyW])), UP);
        for (key, button) in [
            (PhysicalKey::KeyZ, FIRE),
            (PhysicalKey::KeyT, FIRE),
            (PhysicalKey::KeyX, ALT),
            (PhysicalKey::KeyY, ALT),
            (PhysicalKey::Space, 0),
        ] {
            assert_eq!(gm2.local_buttons(&held(&[key])), button, "{key:?}");
        }
        let mut keys = gm2.keys();
        let mut down = down_keys(&keys.events(0, UP | FIRE | ALT));
        down.sort_by_key(|k| *k as u32);
        let mut expected = vec![PhysicalKey::ArrowUp, PhysicalKey::KeyZ, PhysicalKey::KeyX];
        expected.sort_by_key(|k| *k as u32);
        assert_eq!(down, expected);
        let events = keys.events(1, LEFT | FIRE | ALT);
        let mut down = down_keys(&events);
        down.sort_by_key(|k| *k as u32);
        let mut expected = vec![PhysicalKey::KeyA, PhysicalKey::KeyT, PhysicalKey::KeyY];
        expected.sort_by_key(|k| *k as u32);
        assert_eq!(down, expected);
        assert!(events.iter().any(|e| matches!(e,
            PlayerEvent::KeyDown { key } if key.logical_key == LogicalKey::Character('t'))));
    }

    /// Players 3 and 4 use the numpad (GM2's defaults: 111/103/104/105 + 106/109 and
    /// 101/97/98/99 + 96/110); the game sees numpad key codes only with the numpad location.
    #[test]
    fn gun_mayhem_2_players_3_and_4_use_the_numpad() {
        let mut keys = game("gun-mayhem-2").keys();
        let pressed = |events: Vec<PlayerEvent>| -> Vec<(LogicalKey, KeyLocation)> {
            events
                .into_iter()
                .filter_map(|e| match e {
                    PlayerEvent::KeyDown { key } => Some((key.logical_key, key.key_location)),
                    _ => None,
                })
                .collect()
        };
        let numpad = |c| (LogicalKey::Character(c), KeyLocation::Numpad);
        let p3 = pressed(keys.events(2, UP | LEFT | FIRE | ALT));
        for key in [numpad('/'), numpad('7'), numpad('*'), numpad('-')] {
            assert!(p3.contains(&key), "player 3 {key:?}");
        }
        let p4 = pressed(keys.events(3, DOWN | RIGHT | FIRE | ALT));
        for key in [numpad('2'), numpad('3'), numpad('0'), numpad('.')] {
            assert!(p4.contains(&key), "player 4 {key:?}");
        }
        assert_eq!(game("gun-mayhem-2").max_players(), 4);
        assert_eq!(game("bad-ice-cream").max_players(), 2);
    }

    /// Territory War is hot-seat: every player presses the same keys, so a key is down while any
    /// player holds it and released only when nobody does.
    #[test]
    fn territory_war_keys_are_shared_by_every_player() {
        let mut keys = game("territory-war").keys();
        assert_eq!(down_keys(&keys.events(0, LEFT)), vec![PhysicalKey::ArrowLeft]);
        assert!(keys.events(1, LEFT).is_empty(), "already down");
        assert!(keys.events(0, 0).is_empty(), "player 2 still holds it");
        assert_eq!(up_keys(&keys.events(1, 0)), vec![PhysicalKey::ArrowLeft]);
        let events = keys.events(2, KEY1 | VIEW);
        let mut down = down_keys(&events);
        down.sort_by_key(|k| *k as u32);
        let mut expected = vec![PhysicalKey::Digit1, PhysicalKey::KeyV];
        expected.sort_by_key(|k| *k as u32);
        assert_eq!(down, expected);
        // the top-row digit, not the numpad one: the game reads key code 49
        assert!(events.iter().any(|e| matches!(e, PlayerEvent::KeyDown { key }
            if key.logical_key == LogicalKey::Character('1') && key.key_location == KeyLocation::Standard)));
        assert_eq!(game("territory-war").max_players(), 4);
    }

    #[test]
    fn territory_war_local_keys() {
        let tw = game("territory-war");
        for (key, button) in [
            (PhysicalKey::ArrowUp, UP),
            (PhysicalKey::Space, FIRE),
            (PhysicalKey::KeyX, ALT),
            (PhysicalKey::Digit1, KEY1),
            (PhysicalKey::Digit4, KEY4),
            (PhysicalKey::KeyV, VIEW),
            (PhysicalKey::KeyU, QUIT),
            (PhysicalKey::KeyY, YES),
            (PhysicalKey::KeyN, NO),
            (PhysicalKey::KeyW, 0),
        ] {
            assert_eq!(tw.local_buttons(&held(&[key])), button, "{key:?}");
        }
    }

    /// Bow Man: player 1 W/S + Space, player 2 I/K + Enter. The game's own arrow keys zoom, quit
    /// and clear the screen, so the map never presses an arrow.
    #[test]
    fn bow_man_keys_never_press_an_arrow() {
        let bm = game("bow-man");
        let mut keys = bm.keys();
        let mut down = down_keys(&keys.events(0, UP | DOWN | FIRE | LEFT | RIGHT | ALT));
        down.sort_by_key(|k| *k as u32);
        let mut expected = vec![PhysicalKey::KeyW, PhysicalKey::KeyS, PhysicalKey::Space];
        expected.sort_by_key(|k| *k as u32);
        assert_eq!(down, expected);
        let events = keys.events(1, UP | FIRE | LEFT);
        let mut down = down_keys(&events);
        down.sort_by_key(|k| *k as u32);
        let mut expected = vec![PhysicalKey::KeyI, PhysicalKey::Enter];
        expected.sort_by_key(|k| *k as u32);
        assert_eq!(down, expected);
        assert!(events.iter().any(|e| matches!(e,
            PlayerEvent::KeyDown { key } if key.logical_key == LogicalKey::Named(NamedKey::Enter))));
        assert_eq!(bm.max_players(), 2);
        assert_eq!(bm.local_buttons(&held(&[PhysicalKey::ArrowUp])), UP);
        assert_eq!(bm.local_buttons(&held(&[PhysicalKey::KeyS])), DOWN);
        for fire in [PhysicalKey::Space, PhysicalKey::Enter, PhysicalKey::KeyF] {
            assert_eq!(bm.local_buttons(&held(&[fire])), FIRE);
        }
    }

    /// Ultimate Crab Battle: each player has their own move + fire keys (P4 on the numpad), and
    /// every player also presses 1-3, Ctrl, Y and N.
    #[test]
    fn ultimate_crab_battle_keys() {
        let ucb = game("ultimate-crab-battle");
        assert_eq!(ucb.max_players(), 4);
        let mut keys = ucb.keys();
        for (slot, buttons, expected) in [
            (0, UP | FIRE, vec![PhysicalKey::ArrowUp, PhysicalKey::KeyA]),
            (1, LEFT | FIRE, vec![PhysicalKey::KeyJ, PhysicalKey::KeyU]),
            (2, DOWN | FIRE, vec![PhysicalKey::KeyG, PhysicalKey::KeyR]),
            (3, UP | FIRE, vec![PhysicalKey::Numpad8, PhysicalKey::Numpad0]),
        ] {
            let mut down = down_keys(&keys.events(slot, buttons));
            down.sort_by_key(|k| *k as u32);
            let mut expected = expected;
            expected.sort_by_key(|k| *k as u32);
            assert_eq!(down, expected, "slot {slot}");
        }
        for slot in 0..4 {
            let events = keys.events(slot, KEY1 | KEY2 | KEY3 | ALT | YES | NO);
            let mut down = down_keys(&events);
            down.sort_by_key(|k| *k as u32);
            let mut expected = vec![
                PhysicalKey::Digit1,
                PhysicalKey::Digit2,
                PhysicalKey::Digit3,
                PhysicalKey::ControlLeft,
                PhysicalKey::KeyY,
                PhysicalKey::KeyN,
            ];
            expected.sort_by_key(|k| *k as u32);
            assert_eq!(down, expected, "slot {slot}");
            // the top-row digit the game reads as key code 49, not the numpad one
            assert!(events.iter().any(|e| matches!(e, PlayerEvent::KeyDown { key }
                if key.logical_key == LogicalKey::Character('1') && key.key_location == KeyLocation::Standard)));
        }
        for (key, button) in [
            (PhysicalKey::ArrowLeft, LEFT),
            (PhysicalKey::KeyW, UP),
            (PhysicalKey::KeyD, RIGHT),
            (PhysicalKey::KeyA, 0),
            (PhysicalKey::Space, FIRE),
            (PhysicalKey::Enter, FIRE),
            (PhysicalKey::KeyZ, FIRE),
            (PhysicalKey::KeyF, FIRE),
            (PhysicalKey::ControlLeft, ALT),
            (PhysicalKey::Digit3, KEY3),
            (PhysicalKey::KeyY, YES),
            (PhysicalKey::KeyN, NO),
        ] {
            assert_eq!(ucb.local_buttons(&held(&[key])), button, "{key:?}");
        }
    }

    /// AS2 buttons' on(keyPress "1") and text fields hear the typed character, not the key: a
    /// key press also types its character, as a real keyboard does. Named keys (arrows, Enter)
    /// type nothing; buttons match those from the key itself.
    #[test]
    fn a_key_press_types_its_character() {
        let typed = |events: &[PlayerEvent]| -> Vec<char> {
            events
                .iter()
                .filter_map(|e| match e {
                    PlayerEvent::TextInput { codepoint } => Some(*codepoint),
                    _ => None,
                })
                .collect()
        };
        let mut keys = game("ultimate-crab-battle").keys();
        let events = keys.events(0, KEY1);
        assert_eq!(typed(&events), vec!['1']);
        let down = events.iter().position(|e| matches!(e, PlayerEvent::KeyDown { .. }));
        let text = events.iter().position(|e| matches!(e, PlayerEvent::TextInput { .. }));
        assert!(down < text, "the key goes down before its character, as from winit");
        assert!(typed(&keys.events(0, KEY1)).is_empty(), "held: typed once");
        assert!(typed(&keys.events(0, 0)).is_empty(), "a release types nothing");
        assert_eq!(typed(&keys.events(1, FIRE)), vec!['u']);
        assert!(typed(&keys.events(0, UP | ALT)).is_empty(), "arrows and Ctrl type nothing");
        // shared keys type once, when the first player presses them
        let mut keys = game("territory-war").keys();
        assert_eq!(typed(&keys.events(0, KEY1)), vec!['1']);
        assert!(typed(&keys.events(1, KEY1)).is_empty());
    }

    #[test]
    fn games_by_name() {
        for name in ["territory-war", "bad-ice-cream", "gun-mayhem-2", "bow-man", "stealth-hunter-2"] {
            assert_eq!(game(name).name(), name);
        }
        assert!(Game::built_in("pong").is_none());
    }

    #[test]
    fn slot1_maps_to_wasd() {
        let mut keys = game("bad-ice-cream").keys();
        let events = keys.events(1, UP | FIRE);
        let mut down = down_keys(&events);
        down.sort_by_key(|k| *k as u32);
        let mut expected = vec![PhysicalKey::KeyW, PhysicalKey::KeyF];
        expected.sort_by_key(|k| *k as u32);
        assert_eq!(down, expected);
        // the game reads logical keys: W must arrive as the character 'w'
        assert!(events.iter().any(|e| matches!(e,
            PlayerEvent::KeyDown { key } if key.logical_key == LogicalKey::Character('w'))));
    }

    #[test]
    fn slot0_maps_to_arrows_and_space() {
        let mut keys = game("bad-ice-cream").keys();
        let events = keys.events(0, LEFT | FIRE);
        assert!(events.iter().any(|e| matches!(e,
            PlayerEvent::KeyDown { key } if key.logical_key == LogicalKey::Named(NamedKey::ArrowLeft))));
        assert!(down_keys(&events).contains(&PhysicalKey::Space));
    }

    #[test]
    fn only_changes_emitted() {
        let mut keys = game("bad-ice-cream").keys();
        assert_eq!(keys.events(0, UP).len(), 1);
        assert!(keys.events(0, UP).is_empty());
        let events = keys.events(0, UP | RIGHT);
        assert_eq!(down_keys(&events), vec![PhysicalKey::ArrowRight]);
        assert!(up_keys(&events).is_empty());
    }

    #[test]
    fn release_on_zero() {
        let mut keys = game("bad-ice-cream").keys();
        keys.events(1, DOWN | FIRE);
        let events = keys.events(1, 0);
        assert_eq!(up_keys(&events).len(), 2);
        assert!(down_keys(&events).is_empty());
    }

    #[test]
    fn release_all_lets_go_of_every_slot() {
        let mut keys = game("bad-ice-cream").keys();
        keys.events(0, UP);
        keys.events(1, LEFT | FIRE);
        let events = keys.release_all();
        assert_eq!(up_keys(&events).len(), 3);
        assert!(keys.events(0, 0).is_empty());
    }
}
