//! What a game can ask Ruffle Together, the same in AS2 and AS3, through ExternalInterface:
//!
//! ```text
//! ExternalInterface.call("coop.players")         number of players (2-4)
//! ExternalInterface.call("coop.localSlot")       this PC's player slot (0-3): camera, HUD, "you" labels ONLY
//! ExternalInterface.call("coop.mouseX", slot)    that slot's mouse, stage pixels (same space as _root._xmouse)
//! ExternalInterface.call("coop.mouseY", slot)
//! ExternalInterface.call("coop.mouseDown", slot) that slot's left button
//! ```
//!
//! Every answer except `localSlot` is the same on every PC for the same frame (the mice travel
//! with the other inputs). Outside a session there is no provider: `ExternalInterface.available`
//! is false (or the call returns null), so a game falls back to its normal mouse and slot 0.

use crate::coop::lockstep::{InputFrame, MAX_PLAYERS};
use ruffle_core::context::UpdateContext;
use ruffle_core::external::{ExternalInterfaceProvider, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub struct CoopApi {
    pub players: u8,
    pub local_slot: u8,
    /// Each slot's mouse (stage twips) and left button, as of the frame that runs now.
    pub mouse: [(i32, i32, bool); MAX_PLAYERS],
    /// Names the game called (tests check that AS2 and AS3 calls arrive).
    #[cfg(test)]
    pub calls: Vec<String>,
}

impl CoopApi {
    pub fn new(players: u8, local_slot: u8) -> Self {
        Self {
            players,
            local_slot,
            mouse: [(0, 0, false); MAX_PLAYERS],
            #[cfg(test)]
            calls: vec![],
        }
    }

    /// Takes this frame's inputs; a slot without a mouse this frame keeps its last one.
    pub fn update(&mut self, inputs: &[InputFrame]) {
        for (slot, input) in inputs.iter().enumerate().take(MAX_PLAYERS) {
            if let Some(mouse) = input.mouse {
                self.mouse[slot] = mouse;
            }
        }
    }

    pub fn answer(&self, name: &str, args: &[Value]) -> Value {
        let slot = || match args.first() {
            Some(Value::Number(n)) if *n >= 0.0 && (*n as usize) < self.players as usize => Some(*n as usize),
            _ => None,
        };
        match (name, slot()) {
            ("coop.players", _) => Value::Number(self.players.into()),
            ("coop.localSlot", _) => Value::Number(self.local_slot.into()),
            ("coop.mouseX", Some(slot)) => Value::Number(self.mouse[slot].0 as f64 / 20.0),
            ("coop.mouseY", Some(slot)) => Value::Number(self.mouse[slot].1 as f64 / 20.0),
            ("coop.mouseDown", Some(slot)) => Value::Bool(self.mouse[slot].2),
            _ => Value::Undefined,
        }
    }
}

/// Answers the game's ExternalInterface calls from the shared `CoopApi`.
pub struct CoopApiProvider(pub Arc<Mutex<CoopApi>>);

impl ExternalInterfaceProvider for CoopApiProvider {
    fn call_method(&self, _context: &mut UpdateContext<'_>, name: &str, args: &[Value]) -> Value {
        #[cfg(test)]
        if let Ok(mut api) = self.0.lock() {
            api.calls.push(name.to_string());
        }
        self.0.lock().map_or(Value::Undefined, |api| api.answer(name, args))
    }

    fn on_callback_available(&self, _name: &str) {}

    fn get_id(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_in_stage_pixels_per_slot() {
        let mut api = CoopApi::new(3, 2);
        api.update(&[
            InputFrame { buttons: 0, mouse: Some((2000, 4000, false)) },
            InputFrame { buttons: 0, mouse: Some((200, 300, true)) },
            InputFrame::default(),
        ]);
        let n = |v: f64| Value::Number(v);
        assert_eq!(api.answer("coop.players", &[]), n(3.0));
        assert_eq!(api.answer("coop.localSlot", &[]), n(2.0));
        assert_eq!(api.answer("coop.mouseX", &[n(0.0)]), n(100.0));
        assert_eq!(api.answer("coop.mouseY", &[n(1.0)]), n(15.0));
        assert_eq!(api.answer("coop.mouseDown", &[n(1.0)]), Value::Bool(true));
        assert_eq!(api.answer("coop.mouseX", &[n(2.0)]), n(0.0), "slot 2 hasn't moved yet");
        assert_eq!(api.answer("coop.mouseX", &[n(3.0)]), Value::Undefined, "only 3 players");
        assert_eq!(api.answer("coop.mouseX", &[]), Value::Undefined);
        assert_eq!(api.answer("location.href", &[]), Value::Undefined);
        // no mouse this frame: the last one stays
        api.update(&[InputFrame::default(), InputFrame::default(), InputFrame::default()]);
        assert_eq!(api.answer("coop.mouseX", &[n(0.0)]), n(100.0));
    }
}
