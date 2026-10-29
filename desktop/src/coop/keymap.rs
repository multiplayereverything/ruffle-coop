//! Keys in and out. Every player uses their own normal keys (arrows or WASD, plus the game's
//! buttons); the session presses them on everyone's copy as that player's keys in the game.

use crate::coop::lockstep::{ALT, DOWN, FIRE, KEY1, KEY2, KEY3, KEY4, LEFT, NO, QUIT, RIGHT, UP, VIEW, YES};
use ruffle_core::PlayerEvent;
use ruffle_core::events::{KeyDescriptor, KeyLocation, LogicalKey, NamedKey, PhysicalKey};
use std::collections::HashSet;

/// Territory War's own keys, the same for every player (hot-seat).
fn territory_war_button(key: &PhysicalKey) -> u16 {
    match key {
        PhysicalKey::ArrowUp => UP,
        PhysicalKey::ArrowDown => DOWN,
        PhysicalKey::ArrowLeft => LEFT,
        PhysicalKey::ArrowRight => RIGHT,
        PhysicalKey::Space => FIRE,
        PhysicalKey::KeyX => ALT,
        PhysicalKey::Digit1 => KEY1,
        PhysicalKey::Digit2 => KEY2,
        PhysicalKey::Digit3 => KEY3,
        PhysicalKey::Digit4 => KEY4,
        PhysicalKey::KeyV => VIEW,
        PhysicalKey::KeyU => QUIT,
        PhysicalKey::KeyY => YES,
        PhysicalKey::KeyN => NO,
        _ => 0,
    }
}

/// The games a session knows the keys of (`--coop-keys`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    BadIceCream,
    GunMayhem2,
    TerritoryWar,
}

impl Game {
    pub const NAMES: &[&str] = &["bad-ice-cream", "gun-mayhem-2", "territory-war"];

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "bad-ice-cream" => Some(Self::BadIceCream),
            "gun-mayhem-2" => Some(Self::GunMayhem2),
            "territory-war" => Some(Self::TerritoryWar),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::BadIceCream => "bad-ice-cream",
            Self::GunMayhem2 => "gun-mayhem-2",
            Self::TerritoryWar => "territory-war",
        }
    }

    /// How many players the game has keys for.
    pub fn max_players(self) -> u8 {
        self.keys().slots.len() as u8
    }

    /// The keys each player slot presses in the game.
    pub fn keys(self) -> GameKeys {
        match self {
            Self::BadIceCream => GameKeys::bad_ice_cream(),
            Self::GunMayhem2 => GameKeys::gun_mayhem_2(),
            Self::TerritoryWar => GameKeys::territory_war(),
        }
    }
}

/// This PC's buttons from the keys held right now.
pub fn local_buttons(game: Game, held: &HashSet<PhysicalKey>) -> u16 {
    if game == Game::TerritoryWar {
        return held.iter().map(territory_war_button).fold(0, |all, button| all | button);
    }
    let mut buttons = 0;
    for key in held {
        buttons |= match key {
            PhysicalKey::ArrowUp | PhysicalKey::KeyW => UP,
            PhysicalKey::ArrowDown | PhysicalKey::KeyS => DOWN,
            PhysicalKey::ArrowLeft | PhysicalKey::KeyA => LEFT,
            PhysicalKey::ArrowRight | PhysicalKey::KeyD => RIGHT,
            _ => match game {
                Game::BadIceCream => match key {
                    PhysicalKey::Space | PhysicalKey::Enter | PhysicalKey::KeyF => FIRE,
                    _ => 0,
                },
                // either player's set: Z / T shoot, X / Y grenade
                Game::GunMayhem2 => match key {
                    PhysicalKey::KeyZ | PhysicalKey::KeyT => FIRE,
                    PhysicalKey::KeyX | PhysicalKey::KeyY => ALT,
                    _ => 0,
                },
                // handled above
                Game::TerritoryWar => 0,
            },
        };
    }
    buttons
}

#[derive(Clone, Copy)]
struct GameKey {
    button: u16,
    key: KeyDescriptor,
}

fn named(button: u16, physical: PhysicalKey, named: NamedKey) -> GameKey {
    GameKey {
        button,
        key: KeyDescriptor {
            physical_key: physical,
            logical_key: LogicalKey::Named(named),
            key_location: KeyLocation::Standard,
        },
    }
}

fn numpad(button: u16, physical: PhysicalKey, ch: char) -> GameKey {
    GameKey {
        button,
        key: KeyDescriptor {
            physical_key: physical,
            logical_key: LogicalKey::Character(ch),
            key_location: KeyLocation::Numpad,
        },
    }
}

fn character(button: u16, physical: PhysicalKey, ch: char) -> GameKey {
    GameKey {
        button,
        key: KeyDescriptor {
            physical_key: physical,
            logical_key: LogicalKey::Character(ch),
            key_location: KeyLocation::Standard,
        },
    }
}

/// The game's own keys for each player slot, and what each slot holds now.
pub struct GameKeys {
    slots: Vec<Vec<GameKey>>,
    held: Vec<u16>,
    /// Every slot presses the same keys (a hot-seat game): a key is down while any player holds it.
    shared: bool,
}

impl GameKeys {
    /// Bad Ice-Cream: player 1 = arrows + Space, player 2 = W A S D + F.
    pub fn bad_ice_cream() -> Self {
        let p1 = vec![
            named(UP, PhysicalKey::ArrowUp, NamedKey::ArrowUp),
            named(DOWN, PhysicalKey::ArrowDown, NamedKey::ArrowDown),
            named(LEFT, PhysicalKey::ArrowLeft, NamedKey::ArrowLeft),
            named(RIGHT, PhysicalKey::ArrowRight, NamedKey::ArrowRight),
            character(FIRE, PhysicalKey::Space, ' '),
        ];
        let p2 = vec![
            character(UP, PhysicalKey::KeyW, 'w'),
            character(DOWN, PhysicalKey::KeyS, 's'),
            character(LEFT, PhysicalKey::KeyA, 'a'),
            character(RIGHT, PhysicalKey::KeyD, 'd'),
            character(FIRE, PhysicalKey::KeyF, 'f'),
        ];
        Self {
            held: vec![0; 2],
            slots: vec![p1, p2],
            shared: false,
        }
    }

    /// Territory War (hot-seat): arrows, Space (jump, power, shoot), X (cancel the weapon), 1-4
    /// (turn menu), V (map view), U (quit) then Y / N; every player presses the same keys.
    pub fn territory_war() -> Self {
        let keys = vec![
            named(UP, PhysicalKey::ArrowUp, NamedKey::ArrowUp),
            named(DOWN, PhysicalKey::ArrowDown, NamedKey::ArrowDown),
            named(LEFT, PhysicalKey::ArrowLeft, NamedKey::ArrowLeft),
            named(RIGHT, PhysicalKey::ArrowRight, NamedKey::ArrowRight),
            character(FIRE, PhysicalKey::Space, ' '),
            character(ALT, PhysicalKey::KeyX, 'x'),
            character(KEY1, PhysicalKey::Digit1, '1'),
            character(KEY2, PhysicalKey::Digit2, '2'),
            character(KEY3, PhysicalKey::Digit3, '3'),
            character(KEY4, PhysicalKey::Digit4, '4'),
            character(VIEW, PhysicalKey::KeyV, 'v'),
            character(QUIT, PhysicalKey::KeyU, 'u'),
            character(YES, PhysicalKey::KeyY, 'y'),
            character(NO, PhysicalKey::KeyN, 'n'),
        ];
        Self {
            held: vec![0; 4],
            slots: vec![keys; 4],
            shared: true,
        }
    }

    /// Gun Mayhem 2's defaults: player 1 = arrows + Z shoot + X grenade, player 2 = W A S D +
    /// T shoot + Y grenade, player 3 = numpad / 7 8 9 + * shoot + - grenade, player 4 = numpad
    /// 5 1 2 3 + 0 shoot + . grenade.
    pub fn gun_mayhem_2() -> Self {
        let p1 = vec![
            named(UP, PhysicalKey::ArrowUp, NamedKey::ArrowUp),
            named(DOWN, PhysicalKey::ArrowDown, NamedKey::ArrowDown),
            named(LEFT, PhysicalKey::ArrowLeft, NamedKey::ArrowLeft),
            named(RIGHT, PhysicalKey::ArrowRight, NamedKey::ArrowRight),
            character(FIRE, PhysicalKey::KeyZ, 'z'),
            character(ALT, PhysicalKey::KeyX, 'x'),
        ];
        let p2 = vec![
            character(UP, PhysicalKey::KeyW, 'w'),
            character(DOWN, PhysicalKey::KeyS, 's'),
            character(LEFT, PhysicalKey::KeyA, 'a'),
            character(RIGHT, PhysicalKey::KeyD, 'd'),
            character(FIRE, PhysicalKey::KeyT, 't'),
            character(ALT, PhysicalKey::KeyY, 'y'),
        ];
        let p3 = vec![
            numpad(UP, PhysicalKey::NumpadDivide, '/'),
            numpad(LEFT, PhysicalKey::Numpad7, '7'),
            numpad(DOWN, PhysicalKey::Numpad8, '8'),
            numpad(RIGHT, PhysicalKey::Numpad9, '9'),
            numpad(FIRE, PhysicalKey::NumpadMultiply, '*'),
            numpad(ALT, PhysicalKey::NumpadSubtract, '-'),
        ];
        let p4 = vec![
            numpad(UP, PhysicalKey::Numpad5, '5'),
            numpad(LEFT, PhysicalKey::Numpad1, '1'),
            numpad(DOWN, PhysicalKey::Numpad2, '2'),
            numpad(RIGHT, PhysicalKey::Numpad3, '3'),
            numpad(FIRE, PhysicalKey::Numpad0, '0'),
            numpad(ALT, PhysicalKey::NumpadDecimal, '.'),
        ];
        Self {
            held: vec![0; 4],
            slots: vec![p1, p2, p3, p4],
            shared: false,
        }
    }

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

    #[test]
    fn wasd_and_arrows_both_move() {
        let bic = Game::BadIceCream;
        assert_eq!(local_buttons(bic, &held(&[PhysicalKey::KeyW])), UP);
        assert_eq!(local_buttons(bic, &held(&[PhysicalKey::ArrowUp])), UP);
        assert_eq!(
            local_buttons(bic, &held(&[PhysicalKey::KeyA, PhysicalKey::ArrowDown])),
            LEFT | DOWN
        );
        assert_eq!(local_buttons(bic, &held(&[PhysicalKey::KeyD])), RIGHT);
        for fire in [PhysicalKey::Space, PhysicalKey::Enter, PhysicalKey::KeyF] {
            assert_eq!(local_buttons(bic, &held(&[fire])), FIRE);
        }
        assert_eq!(local_buttons(bic, &held(&[PhysicalKey::KeyQ])), 0);
    }

    /// Gun Mayhem 2's own defaults (savedata2.controlarray): player 1 arrows + Z shoot + X
    /// grenade, player 2 W A S D + T shoot + Y grenade; a player may use either set on their PC.
    #[test]
    fn gun_mayhem_2_keys() {
        let gm2 = Game::GunMayhem2;
        assert_eq!(local_buttons(gm2, &held(&[PhysicalKey::ArrowLeft])), LEFT);
        assert_eq!(local_buttons(gm2, &held(&[PhysicalKey::KeyW])), UP);
        for (key, button) in [
            (PhysicalKey::KeyZ, FIRE),
            (PhysicalKey::KeyT, FIRE),
            (PhysicalKey::KeyX, ALT),
            (PhysicalKey::KeyY, ALT),
            (PhysicalKey::Space, 0),
        ] {
            assert_eq!(local_buttons(gm2, &held(&[key])), button, "{key:?}");
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
        let mut keys = Game::GunMayhem2.keys();
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
        assert_eq!(Game::GunMayhem2.max_players(), 4);
        assert_eq!(Game::BadIceCream.max_players(), 2);
    }

    /// Territory War is hot-seat: every player presses the same keys, so a key is down while any
    /// player holds it and released only when nobody does.
    #[test]
    fn territory_war_keys_are_shared_by_every_player() {
        let mut keys = Game::TerritoryWar.keys();
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
        assert_eq!(Game::TerritoryWar.max_players(), 4);
    }

    #[test]
    fn territory_war_local_keys() {
        let tw = Game::TerritoryWar;
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
            assert_eq!(local_buttons(tw, &held(&[key])), button, "{key:?}");
        }
    }

    #[test]
    fn games_by_name() {
        assert_eq!(Game::from_name("territory-war"), Some(Game::TerritoryWar));
        assert_eq!(Game::from_name("bad-ice-cream"), Some(Game::BadIceCream));
        assert_eq!(Game::from_name("gun-mayhem-2"), Some(Game::GunMayhem2));
        assert_eq!(Game::from_name("pong"), None);
        assert_eq!(Game::GunMayhem2.name(), "gun-mayhem-2");
    }

    #[test]
    fn slot1_maps_to_wasd() {
        let mut keys = GameKeys::bad_ice_cream();
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
        let mut keys = GameKeys::bad_ice_cream();
        let events = keys.events(0, LEFT | FIRE);
        assert!(events.iter().any(|e| matches!(e,
            PlayerEvent::KeyDown { key } if key.logical_key == LogicalKey::Named(NamedKey::ArrowLeft))));
        assert!(down_keys(&events).contains(&PhysicalKey::Space));
    }

    #[test]
    fn only_changes_emitted() {
        let mut keys = GameKeys::bad_ice_cream();
        assert_eq!(keys.events(0, UP).len(), 1);
        assert!(keys.events(0, UP).is_empty());
        let events = keys.events(0, UP | RIGHT);
        assert_eq!(down_keys(&events), vec![PhysicalKey::ArrowRight]);
        assert!(up_keys(&events).is_empty());
    }

    #[test]
    fn release_on_zero() {
        let mut keys = GameKeys::bad_ice_cream();
        keys.events(1, DOWN | FIRE);
        let events = keys.events(1, 0);
        assert_eq!(up_keys(&events).len(), 2);
        assert!(down_keys(&events).is_empty());
    }

    #[test]
    fn release_all_lets_go_of_every_slot() {
        let mut keys = GameKeys::bad_ice_cream();
        keys.events(0, UP);
        keys.events(1, LEFT | FIRE);
        let events = keys.release_all();
        assert_eq!(up_keys(&events).len(), 3);
        assert!(keys.events(0, 0).is_empty());
    }
}
