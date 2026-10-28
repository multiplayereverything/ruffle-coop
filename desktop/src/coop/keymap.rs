//! Keys in and out. Every player uses their own normal keys (arrows or WASD, Space/Enter/F);
//! the session presses them on everyone's copy as that player's keys in the game.

use crate::coop::lockstep::{DOWN, FIRE, LEFT, RIGHT, UP};
use ruffle_core::PlayerEvent;
use ruffle_core::events::{KeyDescriptor, KeyLocation, LogicalKey, NamedKey, PhysicalKey};
use std::collections::HashSet;

/// This PC's buttons from the keys held right now.
pub fn local_buttons(held: &HashSet<PhysicalKey>) -> u16 {
    let mut buttons = 0;
    for key in held {
        buttons |= match key {
            PhysicalKey::ArrowUp | PhysicalKey::KeyW => UP,
            PhysicalKey::ArrowDown | PhysicalKey::KeyS => DOWN,
            PhysicalKey::ArrowLeft | PhysicalKey::KeyA => LEFT,
            PhysicalKey::ArrowRight | PhysicalKey::KeyD => RIGHT,
            PhysicalKey::Space | PhysicalKey::Enter | PhysicalKey::KeyF => FIRE,
            _ => 0,
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
        }
    }

    /// Key presses and releases that turn `slot`'s held buttons into `buttons`.
    pub fn events(&mut self, slot: u8, buttons: u16) -> Vec<PlayerEvent> {
        let Some(keys) = self.slots.get(slot as usize) else {
            return vec![];
        };
        let before = std::mem::replace(&mut self.held[slot as usize], buttons);
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
        assert_eq!(local_buttons(&held(&[PhysicalKey::KeyW])), UP);
        assert_eq!(local_buttons(&held(&[PhysicalKey::ArrowUp])), UP);
        assert_eq!(
            local_buttons(&held(&[PhysicalKey::KeyA, PhysicalKey::ArrowDown])),
            LEFT | DOWN
        );
        assert_eq!(local_buttons(&held(&[PhysicalKey::KeyD])), RIGHT);
        for fire in [PhysicalKey::Space, PhysicalKey::Enter, PhysicalKey::KeyF] {
            assert_eq!(local_buttons(&held(&[fire])), FIRE);
        }
        assert_eq!(local_buttons(&held(&[PhysicalKey::KeyQ])), 0);
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
