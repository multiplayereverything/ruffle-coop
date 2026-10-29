//! Runs exactly one frame from every player's input, the same way on every PC. This replaces
//! Ruffle's clock-driven `tick` during a session: frames advance only when the inputs exist,
//! and timers and the frame clock move by exactly one frame each time.

use crate::coop::api::{CoopApi, CoopApiProvider};
use crate::coop::keymap::{Game, GameKeys};
use crate::coop::lockstep::{InputFrame, typed_event};
use ruffle_core::events::MouseButton;
use ruffle_core::limits::ExecutionLimit;
use ruffle_core::{FloatDuration, Player, PlayerEvent};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Stepper {
    keys: GameKeys,
    mouse: Option<(i32, i32, bool)>,
    loaded: bool,
    /// The key map the game gave with `coop.keys`, if it gave one.
    game_keys: Option<String>,
    /// What the game can ask through ExternalInterface (every slot's mouse, the players).
    api: Arc<Mutex<CoopApi>>,
}

impl Stepper {
    pub fn new(keys: GameKeys, players: u8, local_slot: u8) -> Self {
        Self {
            keys,
            mouse: None,
            loaded: false,
            game_keys: None,
            api: Arc::new(Mutex::new(CoopApi::new(players, local_slot))),
        }
    }

    #[cfg(test)]
    pub fn api(&self) -> &Arc<Mutex<CoopApi>> {
        &self.api
    }

    /// `inputs` is every player's input for this frame, by player index; only the host's
    /// (index 0) mouse is used.
    pub fn step(&mut self, player: &mut Player, inputs: &[InputFrame]) {
        if !self.loaded {
            // frames load part of a streaming movie within a time limit, so a slower PC would
            // load less per frame; load all of it before the first frame instead
            while !player.preload(&mut ExecutionLimit::none()) {}
            self.loaded = true;
            player.set_external_interface_provider(Some(Box::new(CoopApiProvider(self.api.clone()))));
        }
        // the host's mouse is always in the stage, wherever this PC's own cursor is
        // every slot's mouse, for the game's ExternalInterface calls during this frame
        if let Ok(mut api) = self.api.lock() {
            api.update(inputs);
        }
        player.set_mouse_in_stage(true);
        for (slot, input) in inputs.iter().enumerate() {
            for event in self.keys.events(slot as u8, input.buttons) {
                player.handle_event(event);
            }
            // what each player typed, in slot order: names, chat, keyPress buttons
            for &c in input.text.chars() {
                player.handle_event(typed_event(c));
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
        self.apply_game_calls(player);
        player.audio_mut().tick();
        player.advance_frame_clock(frame);
        // rendering has side effects on the game (e.g. scrollRect updates), so it happens
        // exactly once per frame on every PC
        player.render();
    }

    /// Something the game hears through `coop.events` ("join 1", "leave 2", "chat 0 hi"...);
    /// every PC adds the same on the same frame.
    pub fn add_event(&mut self, event: String) {
        if let Ok(mut api) = self.api.lock() {
            api.add_event(event);
        }
    }

    /// Whether the game reads `coop.typed` (every typed character is wanted then).
    pub fn wants_text(&self) -> bool {
        self.api.lock().is_ok_and(|api| api.wants_text())
    }

    /// This PC's round trip to each player, for `coop.ping`.
    pub fn set_pings(&mut self, pings: [Option<u32>; crate::coop::lockstep::MAX_PLAYERS]) {
        if let Ok(mut api) = self.api.lock() {
            api.pings = pings;
        }
    }

    /// The host's game settings, for `coop.option`.
    pub fn set_options(&mut self, options: &[(String, String)]) {
        if let Ok(mut api) = self.api.lock() {
            api.options = options.to_vec();
        }
    }

    /// Every player's name, for `coop.name`.
    pub fn set_names(&mut self, names: &[String]) {
        if let Ok(mut api) = self.api.lock() {
            api.names = names.to_vec();
        }
    }

    /// The labels the game asked for, for the window to draw.
    pub fn labels(&self) -> Vec<crate::coop::api::Label> {
        self.api.lock().map(|api| api.labels.iter().flatten().cloned().collect()).unwrap_or_default()
    }

    /// What the game asked for during the frame (its scripts and timers): its own key map (every
    /// PC switches on the same frame, since every PC runs the same calls; the same map again
    /// changes nothing) and this PC's camera.
    fn apply_game_calls(&mut self, player: &mut Player) {
        let Ok(mut api) = self.api.lock() else {
            return;
        };
        if let Some(text) = api.keys.take()
            && self.game_keys.as_ref() != Some(&text)
            && let Ok(game) = Game::parse("game", &text)
        {
            self.game_keys = Some(text);
            for event in self.keys.release_all() {
                player.handle_event(event);
            }
            self.keys = game.keys();
        }
        player.set_coop_camera(api.local_camera().map(|c| (c.x, c.y, c.zoom)));
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
    use crate::coop::keymap::Game;
    use crate::coop::api::Camera;
    use crate::coop::lockstep::{FIRE, UP};
    use ruffle_core::PlayerBuilder;
    use ruffle_core::external::Value;
    use ruffle_core::frame_clock::FrameClock;
    use std::sync::{Arc, Mutex};

    #[test]
    fn step_advances_clock_by_exactly_one_frame() {
        let player = PlayerBuilder::new().build();
        let mut player = player.lock().expect("player");
        let clock = Arc::new(Mutex::new(FrameClock::new(0)));
        player.set_frame_clock(Some(clock.clone()));
        let frame = Duration::from_secs_f64(1.0 / player.frame_rate());

        let mut stepper = Stepper::new(Game::built_in("bad-ice-cream").expect("built in").keys(), 2, 0);
        let inputs = [InputFrame { buttons: UP, mouse: Some((200, 400, true)), ..Default::default() }, InputFrame::default()];
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
            let mut stepper = Stepper::new(Game::built_in("bad-ice-cream").expect("built in").keys(), 2, 0);
            let inputs = [InputFrame { buttons: 0, mouse: Some((4001, 3999, false)), ..Default::default() }];
            stepper.step(&mut player, &inputs);
            assert!(player.mouse_in_stage(), "the host's mouse is always in the stage");
            spots.push(player.mouse_position_twips());
        }
        assert_eq!(spots, vec![(4001, 3999); 3]);
    }

    /// The co-op API answers through ExternalInterface; Ruffle's own AS2 and AS3 ExternalInterface
    /// tests show that both kinds of game reach it in a session.
    #[test]
    fn as2_and_as3_reach_the_coop_api() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/tests/swfs");
        for swf in ["avm1/external_interface/test.swf", "avm2/external_interface/test.swf"] {
            let player = crate::coop::replay::headless_player(
                &root.join(swf),
                None,
                crate::coop::storage::SessionStorage::default(),
            )
            .expect("player");
            let mut player = player.lock().expect("player");
            let mut stepper = Stepper::new(Game::built_in("bad-ice-cream").expect("built in").keys(), 2, 0);
            for _ in 0..5 {
                stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
            }
            let calls = stepper.api().lock().expect("api").calls.clone();
            assert!(calls.contains(&"ping".to_string()), "{swf}: {calls:?}");
        }
    }

    struct Traces(Arc<Mutex<Vec<String>>>);

    impl ruffle_core::backend::log::LogBackend for Traces {
        fn avm_trace(&self, message: &str) {
            self.0.lock().expect("traces").push(message.to_string());
        }

        fn avm_warning(&self, _message: &str) {}
    }

    /// The frames at which Ruffle's own AVM2 sound-complete test SWF (two embedded sounds) sees
    /// each channel's soundComplete, without an audio device (as on a PC without one).
    fn sound_complete_frames(session: bool) -> Vec<(usize, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/tests/swfs");
        let movie = ruffle_core::tag_utils::movie_from_path(
            root.join("avm2/soundchannel_soundcomplete/test.swf"),
            None,
        )
        .expect("test swf");
        let traces = Arc::new(Mutex::new(vec![]));
        let player = PlayerBuilder::new()
            .with_movie(movie)
            .with_log(Traces(traces.clone()))
            .with_autoplay(true)
            .build();
        let mut player = player.lock().expect("player");
        if session {
            player.set_frame_clock(Some(Arc::new(Mutex::new(FrameClock::new(0)))));
        }
        let mut stepper = Stepper::new(Game::built_in("bad-ice-cream").expect("built in").keys(), 2, 0);
        let mut seen = vec![];
        for frame in 0..300 {
            stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
            for line in traces.lock().expect("traces").drain(..) {
                if line.contains("soundComplete") {
                    seen.push((frame, line));
                }
            }
        }
        seen
    }

    /// A sound's end follows the frame clock in a session, so soundComplete / onSoundComplete
    /// fire at the same frame on every PC, whatever (or whether) its audio device plays.
    #[test]
    fn sounds_complete_on_the_frame_clock() {
        let seen = sound_complete_frames(true);
        let channels: Vec<_> = seen.iter().map(|(_, line)| line.split(':').next().unwrap_or("")).collect();
        assert_eq!(channels, ["noise_channel", "silence_channel"], "{seen:?}");
        assert_eq!(seen, sound_complete_frames(true), "the same frames every time");

        // control: outside a session a PC without an audio device never ends a sound
        assert_eq!(sound_complete_frames(false), vec![]);
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
        let mut stepper = Stepper::new(Game::built_in("bad-ice-cream").expect("built in").keys(), 2, 0);
        stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
        assert!(player.preload(&mut ruffle_core::limits::ExecutionLimit::exhausted()));
    }

    /// Ruffle's own AS2 text-input test: a selected text box, traced on Escape. Two players type
    /// into it (in slot order, on the same frame), Backspace included, then Escape shows it.
    #[test]
    fn every_players_typing_reaches_the_selected_text_box() {
        use crate::coop::lockstep::{Typed, text_control_char};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/tests/swfs");
        let movie =
            ruffle_core::tag_utils::movie_from_path(root.join("avm1/edittext_input/test.swf"), None).expect("test swf");
        let traces = Arc::new(Mutex::new(vec![]));
        let player = PlayerBuilder::new()
            .with_movie(movie)
            .with_log(Traces(traces.clone()))
            .with_autoplay(true)
            .build();
        let mut player = player.lock().expect("player");
        let escape = Game::parse("t", "slot 0: FIRE=27\nslot 1: FIRE=27").expect("map").keys();
        let mut stepper = Stepper::new(escape, 2, 0);
        stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
        assert!(player.coop_text_box_focused(), "the SWF selects its text box");
        let typed = |text: &str| InputFrame { text: Typed::from_chars(text.chars()), ..Default::default() };
        let backspace = Typed::from_chars([text_control_char(ruffle_core::events::TextControlCode::Backspace)]);
        stepper.step(&mut player, &[typed("Lia"), typed("xm")]);
        stepper.step(&mut player, &[InputFrame::default(), InputFrame { text: backspace, ..Default::default() }]);
        stepper.step(&mut player, &[typed("!"), InputFrame::default()]);
        stepper.step(&mut player, &[InputFrame { buttons: FIRE, ..Default::default() }, InputFrame::default()]);
        assert_eq!(traces.lock().expect("traces").last().map(String::as_str), Some("Liax!"));
    }

    /// Bad Ice-Cream, headless (needs COOP_TEST_SWF), and a Stepper for `local_slot`.
    fn bic(local_slot: u8) -> Option<(Arc<Mutex<Player>>, Stepper)> {
        let swf = std::env::var("COOP_TEST_SWF").ok()?;
        let player = crate::coop::replay::headless_player(
            std::path::Path::new(&swf),
            Some(Arc::new(Mutex::new(FrameClock::new(1_700_000_000_000)))),
            crate::coop::storage::SessionStorage::default(),
        )
        .expect("player");
        let keys = Game::built_in("bad-ice-cream").expect("built in").keys();
        Some((player, Stepper::new(keys, 2, local_slot)))
    }

    fn set_cameras(stepper: &Stepper, cameras: [Option<Camera>; 2]) {
        let mut api = stepper.api().lock().expect("api");
        api.cameras[..2].copy_from_slice(&cameras);
    }

    /// The share of pixels that differ between two pictures of the same size.
    fn differ(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
        assert_eq!(a.dimensions(), b.dimensions());
        let changed = a.pixels().zip(b.pixels()).filter(|(p, q)| p != q).count();
        changed as f64 / (a.width() * a.height()) as f64
    }

    /// Every slot has its own camera and each PC draws with its own: the pictures differ, the
    /// game (its fingerprint) doesn't. A HUD clip is drawn as if there were no camera.
    #[test]
    fn cameras_change_the_picture_not_the_game() {
        let Some((host, mut host_stepper)) = bic(0) else {
            eprintln!("skipping: COOP_TEST_SWF not set");
            return;
        };
        let (friend, mut friend_stepper) = bic(1).expect("player");
        let (plain, mut plain_stepper) = bic(0).expect("player");
        let cameras = [
            Some(Camera { x: 100.0, y: 100.0, zoom: 2.0 }),
            Some(Camera { x: 400.0, y: 300.0, zoom: 1.5 }),
        ];
        set_cameras(&host_stepper, cameras);
        set_cameras(&friend_stepper, cameras);
        let (mut host, mut friend, mut plain) =
            (host.lock().expect("lock"), friend.lock().expect("lock"), plain.lock().expect("lock"));
        let inputs = [InputFrame { buttons: UP, mouse: None, ..Default::default() }, InputFrame::default()];
        for frame in 0..120 {
            host_stepper.step(&mut host, &inputs);
            friend_stepper.step(&mut friend, &inputs);
            plain_stepper.step(&mut plain, &inputs);
            let print = plain.coop_fingerprint();
            assert_eq!(host.coop_fingerprint(), print, "host, frame {frame}");
            assert_eq!(friend.coop_fingerprint(), print, "friend, frame {frame}");
        }
        let capture = crate::coop::replay::capture;
        let (host_picture, friend_picture, plain_picture) = (
            capture(&mut host).expect("capture"),
            capture(&mut friend).expect("capture"),
            capture(&mut plain).expect("capture"),
        );
        assert!(differ(&host_picture, &plain_picture) > 0.2, "the host's camera moved the picture");
        assert!(differ(&friend_picture, &host_picture) > 0.2, "each PC draws its own slot's camera");

        // the whole root as HUD: drawn without the camera, so like no camera at all
        host.mutate_with_update_context(|context| {
            assert!(ruffle_core::coop_view::set_hud(context, "_root"));
        });
        let hud_picture = capture(&mut host).expect("capture");
        assert!(differ(&hud_picture, &plain_picture) < 0.01, "{}", differ(&hud_picture, &plain_picture));
        assert_eq!(host.coop_fingerprint(), plain.coop_fingerprint(), "HUD doesn't touch the game");
    }

    /// This PC's cursor reaches the game through its camera: the middle of the window is the
    /// point the camera looks at, in stage twips, on every PC.
    #[test]
    fn the_mouse_goes_through_the_camera() {
        let Some((player, mut stepper)) = bic(0) else {
            eprintln!("skipping: COOP_TEST_SWF not set");
            return;
        };
        set_cameras(&stepper, [Some(Camera { x: 100.0, y: 50.0, zoom: 2.0 }), None]);
        let mut player = player.lock().expect("lock");
        stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
        let (width, height) = (player.movie_width() as f64, player.movie_height() as f64);
        let (x, y) = player.viewport_to_stage(width / 2.0, height / 2.0);
        assert!((x - 2000).abs() <= 1 && (y - 1000).abs() <= 1, "({x}, {y})");
        let label = player.coop_label_position("_root").expect("the root is on show");
        player.set_coop_camera(None);
        assert_ne!(player.coop_label_position("_root"), Some(label), "labels follow the camera");
    }

    /// `coop.keys`: the game's own map is used from the next frame, held keys are let go first,
    /// and giving the same map again (every frame, say) changes nothing.
    #[test]
    fn the_game_can_switch_the_key_map() {
        let player = PlayerBuilder::new().build();
        let mut player = player.lock().expect("player");
        let mut stepper = Stepper::new(Game::built_in("bad-ice-cream").expect("built in").keys(), 2, 0);
        let map = "slot 0: FIRE=65\nslot 1: FIRE=66";
        let give = |stepper: &Stepper| {
            let answer = stepper.api().lock().expect("api").answer("coop.keys", &[Value::String(map.into())]);
            assert_eq!(answer, Value::Bool(true));
        };
        give(&stepper);
        let fire = [InputFrame { buttons: FIRE, mouse: None, ..Default::default() }, InputFrame::default()];
        stepper.step(&mut player, &fire); // the switch happens at the end of this frame
        assert_eq!(stepper.game_keys.as_deref(), Some(map));
        // the new map: slot 0's FIRE is A now (it was Space), and nothing is held yet
        let events = stepper.keys.events(0, FIRE);
        assert!(matches!(events.first(), Some(PlayerEvent::KeyDown { key })
            if key.physical_key == ruffle_core::events::PhysicalKey::KeyA), "{events:?}");
        give(&stepper);
        stepper.step(&mut player, &fire);
        assert!(stepper.keys.events(0, FIRE).is_empty(), "FIRE stays held: the same map didn't reset it");
    }
}
