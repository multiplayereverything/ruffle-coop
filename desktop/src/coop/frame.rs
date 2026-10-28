//! Runs exactly one frame from every player's input, the same way on every PC. This replaces
//! Ruffle's clock-driven `tick` during a session: frames advance only when the inputs exist,
//! and timers and the frame clock move by exactly one frame each time.

use crate::coop::keymap::GameKeys;
use crate::coop::lockstep::InputFrame;
use ruffle_core::events::MouseButton;
use ruffle_core::limits::ExecutionLimit;
use ruffle_core::{FloatDuration, Player, PlayerEvent};
use std::time::Duration;

pub struct Stepper {
    keys: GameKeys,
    mouse: Option<(i32, i32, bool)>,
    loaded: bool,
}

impl Stepper {
    pub fn new(keys: GameKeys) -> Self {
        Self {
            keys,
            mouse: None,
            loaded: false,
        }
    }

    /// `inputs` is every player's input for this frame, by player index; only the host's
    /// (index 0) mouse is used.
    pub fn step(&mut self, player: &mut Player, inputs: &[InputFrame]) {
        if !self.loaded {
            // frames load part of a streaming movie within a time limit, so a slower PC would
            // load less per frame; load all of it before the first frame instead
            while !player.preload(&mut ExecutionLimit::none()) {}
            self.loaded = true;
        }
        // the host's mouse is always in the stage, wherever this PC's own cursor is
        player.set_mouse_in_stage(true);
        for (slot, input) in inputs.iter().enumerate() {
            for event in self.keys.events(slot as u8, input.buttons) {
                player.handle_event(event);
            }
        }
        let mouse = inputs.first().and_then(|input| input.mouse);
        if mouse != self.mouse
            && let Some((x, y, down)) = mouse
        {
            // inputs carry stage positions, used exactly as they are on every PC (the window
            // position below is only for Ruffle's event plumbing)
            player.set_stage_mouse_override(Some((x, y)));
            let (x, y) = player.stage_to_viewport(x, y);
            player.handle_event(PlayerEvent::MouseMove { x, y });
            let was_down = self.mouse.is_some_and(|(_, _, down)| down);
            let button = MouseButton::Left;
            if down && !was_down {
                player.handle_event(PlayerEvent::MouseDown {
                    x,
                    y,
                    button,
                    index: Some(0),
                });
            } else if was_down && !down {
                player.handle_event(PlayerEvent::MouseUp { x, y, button });
            }
        }
        self.mouse = mouse;

        let frame = Duration::from_secs_f64(1.0 / player.frame_rate());
        player.run_frame();
        player.update_timers(FloatDuration::from_std(frame));
        player.audio_mut().tick();
        player.advance_frame_clock(frame);
        // rendering has side effects on the game (e.g. scrollRect updates), so it happens
        // exactly once per frame on every PC
        player.render();
    }

    /// Lets go of every key, e.g. when the session ends.
    pub fn release_all(&mut self, player: &mut Player) {
        for event in self.keys.release_all() {
            player.handle_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coop::lockstep::UP;
    use ruffle_core::PlayerBuilder;
    use ruffle_core::frame_clock::FrameClock;
    use std::sync::{Arc, Mutex};

    #[test]
    fn step_advances_clock_by_exactly_one_frame() {
        let player = PlayerBuilder::new().build();
        let mut player = player.lock().expect("player");
        let clock = Arc::new(Mutex::new(FrameClock::new(0)));
        player.set_frame_clock(Some(clock.clone()));
        let frame = Duration::from_secs_f64(1.0 / player.frame_rate());

        let mut stepper = Stepper::new(GameKeys::bad_ice_cream());
        let inputs = [InputFrame { buttons: UP, mouse: Some((200, 400, true)) }, InputFrame::default()];
        stepper.step(&mut player, &inputs);
        stepper.step(&mut player, &inputs);

        assert_eq!(clock.lock().expect("clock").elapsed(), frame * 2);
    }

    /// Two PCs with different (or minimized, 1x1) windows must see the host's mouse at exactly
    /// the same stage spot, and inside the stage, whatever their own cursor does.
    #[test]
    fn host_mouse_is_the_same_on_every_pc() {
        let mut spots = vec![];
        for (width, height) in [(550, 550), (1, 1), (1280, 720)] {
            let player = PlayerBuilder::new().with_viewport_dimensions(width, height, 1.0).build();
            let mut player = player.lock().expect("player");
            player.set_mouse_in_stage(false); // this PC's own cursor is outside its window
            let mut stepper = Stepper::new(GameKeys::bad_ice_cream());
            let inputs = [InputFrame { buttons: 0, mouse: Some((4001, 3999, false)) }];
            stepper.step(&mut player, &inputs);
            assert!(player.mouse_in_stage(), "the host's mouse is always in the stage");
            spots.push(player.mouse_position_twips());
        }
        assert_eq!(spots, vec![(4001, 3999); 3]);
    }

    /// Streaming loads only part of a movie per frame, as fast as the PC allows; the first
    /// step loads all of it, so every PC starts from the same fully loaded movie.
    #[test]
    fn first_step_loads_the_whole_movie() {
        let Ok(swf) = std::env::var("COOP_TEST_SWF") else {
            eprintln!("skipping: COOP_TEST_SWF not set");
            return;
        };
        let player = crate::coop::replay::headless_player(
            std::path::Path::new(&swf),
            None,
            crate::coop::storage::SessionStorage::default(),
        )
        .expect("player");
        let mut player = player.lock().expect("player");
        let mut stepper = Stepper::new(GameKeys::bad_ice_cream());
        stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
        assert!(player.preload(&mut ruffle_core::limits::ExecutionLimit::exhausted()));
    }
}
