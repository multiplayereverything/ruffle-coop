//! Runs exactly one frame from every player's input, the same way on every PC. This replaces
//! Ruffle's clock-driven `tick` during a session: frames advance only when the inputs exist,
//! and timers and the frame clock move by exactly one frame each time.

use crate::coop::api::{CoopApi, CoopApiProvider, Private};
use crate::coop::keymap::{Game, GameKeys, Hover, MouseMode, Zone};
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
    /// Each slot's last mouse, and whose click the game's one mouse button is down for.
    mice: [Option<(i32, i32, bool)>; crate::coop::lockstep::MAX_PLAYERS],
    clicking: Option<u8>,
    /// Whose click reached the game last: a private screen that opens belongs to them.
    last_clicker: Option<u8>,
    /// The game's mouse is resting off the stage (no hover for anyone).
    parked: bool,
    /// This PC's own mouse right now (stage twips), for this player's own hover.
    local_mouse: Option<(i32, i32)>,
    /// Clicks refused because another player has that private screen open: (who, whose).
    busy: Vec<(u8, u8)>,
    /// Friends' clicks waiting for the game's one mouse button, oldest first.
    queue: std::collections::VecDeque<QueuedClick>,
    /// A queued click whose player had already let go: released on the next frame (who, where).
    release_next: Option<(u8, i32, i32)>,
    /// How many frames the current friend's click has been held while others wait.
    held: u32,
}

/// A friend's click waiting for the game's one mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueuedClick {
    slot: u8,
    x: i32,
    y: i32,
    /// They let go while waiting: the click is a press and a release.
    released: bool,
}

/// The most clicks that wait; more are dropped (nobody clicks that fast).
const QUEUE_MAX: usize = 8;

/// A friend may hold the game's mouse button this long (frames) while others wait; then their
/// click is let go, so nobody blocks the others.
const HOLD_MAX: u32 = 30;

/// Where the game's mouse rests while nobody's hover should show: off the stage (twips).
const PARKED: (i32, i32) = (-4000, -4000);

impl Stepper {
    pub fn new(keys: GameKeys, players: u8, local_slot: u8) -> Self {
        let mut api = CoopApi::new(players, local_slot);
        api.mouse_mode = keys.mouse.unwrap_or_default();
        api.zones = keys.zones.clone();
        api.privates = privates(&keys.privates);
        api.hover = keys.hover;
        Self {
            keys,
            mouse: None,
            loaded: false,
            game_keys: None,
            api: Arc::new(Mutex::new(api)),
            mice: [None; crate::coop::lockstep::MAX_PLAYERS],
            clicking: None,
            last_clicker: None,
            parked: false,
            local_mouse: None,
            busy: vec![],
            queue: Default::default(),
            release_next: None,
            held: 0,
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
        // Characters reach the game from one place only, or a key would type twice: while a text
        // box is selected, what the players typed (names, chat); otherwise each slot's key press
        // types its game key's character (on(keyPress "a") buttons), whatever key map it comes
        // from and whoever pressed it (keyboard, script, replay). Game state, so the same on
        // every PC.
        let text_box = player.coop_text_box_focused();
        for (slot, input) in inputs.iter().enumerate() {
            for event in self.keys.events(slot as u8, input.buttons) {
                if !(text_box && matches!(event, PlayerEvent::TextInput { .. })) {
                    player.handle_event(event);
                }
            }
            if text_box {
                for &c in input.text.chars() {
                    player.handle_event(typed_event(c));
                }
            }
        }
        self.feed_mice(player, inputs);

        let frame = Duration::from_secs_f64(1.0 / player.frame_rate());
        player.run_frame();
        player.update_timers(FloatDuration::from_std(frame));
        self.apply_game_calls(player);
        self.update_privates(player);
        self.show_own_hover(player);
        player.audio_mut().tick();
        player.advance_frame_clock(frame);
        // rendering has side effects on the game (e.g. scrollRect updates), so it happens
        // exactly once per frame on every PC
        player.render();
    }

    /// The game has one mouse. It rests where the host's is; each player's click (the host's
    /// first) moves it to that player's point for the click, and back to the host's after. Who
    /// may click is the key map's or the game's rule: the host only, friends too on what the game
    /// made clickable (anywhere, or only in their own zone), or anyone anywhere; nobody in a box
    /// that belongs to someone else. Flash has one mouse button, so one click at a time: the
    /// host's always goes at once, a friend's waits its turn (oldest first, then by slot), and a
    /// friend holding the button long while others wait is let go. Everything comes from inputs
    /// and game state, so every PC does the same.
    fn feed_mice(&mut self, player: &mut Player, inputs: &[InputFrame]) {
        let (mode, mut zones, privates, hover) = self
            .api
            .lock()
            .map(|api| (api.mouse_mode, api.zones.clone(), api.privates.clone(), api.hover))
            .unwrap_or_default();
        // an open private screen is its owner's alone
        zones.extend(owned_zones(&privates));
        let rules = ClickRules { mode, zones, privates };
        // `hover: off`: the game's mouse rests off the stage between clicks, so there is no hover
        // at all; otherwise it follows the host's, as in the game (the default; with `own`, each
        // PC then draws Button symbols by its own player's mouse)
        let park = mode != MouseMode::Host && hover == Some(Hover::Off);
        let host = inputs.first().and_then(|input| input.mouse);
        if park {
            if !self.parked && self.clicking.is_none() {
                move_mouse(player, PARKED.0, PARKED.1);
                self.parked = true;
            }
        } else if host != self.mouse
            && let Some((x, y, _)) = host
            && self.clicking.is_none_or(|slot| slot == 0)
        {
            as_player(player, &rules.privates, 0, |player| move_mouse(player, x, y));
            self.parked = false;
        }
        self.mouse = host;
        // a queued click whose player had already let go: its release
        if let Some((slot, x, y)) = self.release_next.take()
            && self.clicking == Some(slot)
        {
            self.end_click(player, &rules, park, slot, x, y);
        }
        for (slot, input) in inputs.iter().enumerate().take(self.mice.len()) {
            let before = self.mice[slot];
            let Some((x, y, down)) = input.mouse else {
                continue;
            };
            self.mice[slot] = input.mouse;
            let was_down = before.is_some_and(|(_, _, down)| down);
            let slot = slot as u8;
            if self.clicking == Some(slot) {
                if (slot > 0 || park) && before.is_some_and(|(bx, by, _)| (bx, by) != (x, y)) {
                    as_player(player, &rules.privates, slot, |player| move_mouse(player, x, y)); // a drag
                }
                if !down {
                    self.end_click(player, &rules, park, slot, x, y);
                }
            } else if down && !was_down {
                if slot == 0 {
                    if let Some(friend) = self.clicking.take() {
                        // the host never waits on a friend: a friend's click still held is let go
                        // where the host clicks, then the host's click goes through
                        as_player(player, &rules.privates, friend, |player| {
                            move_mouse(player, x, y);
                            let (vx, vy) = player.stage_to_viewport(x, y);
                            player.handle_event(PlayerEvent::MouseUp { x: vx, y: vy, button: MouseButton::Left });
                        });
                        self.release_next = None;
                        tracing::info!("Ruffle Together: player {}'s held click let go for the host's", friend + 1);
                    }
                    self.start_click(player, &rules, park, 0, x, y);
                } else if self.clicking.is_none() && self.queue.is_empty() {
                    self.start_click(player, &rules, park, slot, x, y);
                } else if self.queue.len() < QUEUE_MAX {
                    // the mouse is busy: this click waits its turn
                    self.queue.push_back(QueuedClick { slot, x, y, released: false });
                }
            } else if !down && was_down {
                // let go while waiting: the click is still theirs, as a press and a release
                if let Some(waiting) = self.queue.iter_mut().rev().find(|q| q.slot == slot && !q.released) {
                    waiting.released = true;
                }
            }
        }
        // a friend holding the button long while others wait is let go
        match self.clicking {
            Some(slot) if slot > 0 && !self.queue.is_empty() => {
                self.held += 1;
                if self.held > HOLD_MAX
                    && let Some((x, y, _)) = self.mice[slot as usize]
                {
                    tracing::info!("Ruffle Together: player {}'s long click let go for the others", slot + 1);
                    self.end_click(player, &rules, park, slot, x, y);
                }
            }
            _ => self.held = 0,
        }
        // the next waiting clicks, when the mouse is free (a refused one doesn't use it)
        while self.clicking.is_none()
            && self.release_next.is_none()
            && let Some(next) = self.queue.pop_front()
        {
            if self.start_click(player, &rules, park, next.slot, next.x, next.y) && next.released {
                self.release_next = Some((next.slot, next.x, next.y));
            }
        }
    }

    /// Presses the game's mouse button for `slot` at (x, y) (twips) if the rules let them click
    /// there; false if refused.
    fn start_click(&mut self, player: &mut Player, rules: &ClickRules, park: bool, slot: u8, x: i32, y: i32) -> bool {
        let (allowed, target) = as_player(player, &rules.privates, slot, |player| {
            (may_click(player, rules.mode, &rules.zones, slot, x, y), player.coop_click_target(x, y))
        });
        // the game has one of each private screen: while it is someone's, nobody else opens it
        // (that would take it over)
        let busy = target.as_deref().and_then(|target| {
            rules.privates.iter().find_map(|p| {
                let opener = p.openers.as_deref().is_some_and(|pattern| crate::coop::keymap::path_fits(target, pattern));
                p.owner.filter(|&owner| owner != slot && opener)
            })
        });
        if let Some(owner) = busy {
            self.busy.push((slot, owner));
        }
        let allowed = allowed && busy.is_none();
        if rules.mode != MouseMode::Host || !rules.privates.is_empty() {
            // for key map authors: what each click lands on (zones and private screens name these
            // paths)
            let target = target.unwrap_or_else(|| "nothing clickable".into());
            let verdict = if allowed { "clicks" } else { "may not click" };
            tracing::info!("Ruffle Together: player {} {verdict} {target} ({}, {})", slot + 1, x / 20, y / 20);
        }
        if allowed {
            as_player(player, &rules.privates, slot, |player| {
                if slot > 0 || park {
                    move_mouse(player, x, y);
                }
                let (vx, vy) = player.stage_to_viewport(x, y);
                player.handle_event(PlayerEvent::MouseDown { x: vx, y: vy, button: MouseButton::Left, index: Some(0) });
            });
            self.clicking = Some(slot);
            self.last_clicker = Some(slot);
            self.parked = false;
            self.held = 0;
        }
        allowed
    }

    /// Lets go of `slot`'s click at (x, y) (twips), then the game's mouse goes back to the host's
    /// (or off the stage).
    fn end_click(&mut self, player: &mut Player, rules: &ClickRules, park: bool, slot: u8, x: i32, y: i32) {
        as_player(player, &rules.privates, slot, |player| {
            let (vx, vy) = player.stage_to_viewport(x, y);
            player.handle_event(PlayerEvent::MouseUp { x: vx, y: vy, button: MouseButton::Left });
        });
        self.clicking = None;
        self.held = 0;
        if park {
            move_mouse(player, PARKED.0, PARKED.1);
            self.parked = true;
        } else if slot > 0
            && let Some((hx, hy, _)) = self.mouse
        {
            as_player(player, &rules.privates, 0, |player| move_mouse(player, hx, hy));
        }
    }

    /// The other players' mice (slot, stage twips, button) for the window to draw, only while
    /// one is over something that player may click (their own card on a character screen); none
    /// at all while only the host clicks. The picture only.
    pub fn friend_cursors(&self, player: &mut Player) -> Vec<(u8, i32, i32, bool)> {
        let Ok((mode, mut zones, privates, mice, players, local)) = self.api.lock().map(|api| {
            (api.mouse_mode, api.zones.clone(), api.privates.clone(), api.mouse, api.players, api.local_slot)
        }) else {
            return vec![];
        };
        if mode == MouseMode::Host {
            return vec![];
        }
        zones.extend(owned_zones(&privates));
        (0..players)
            .filter(|&slot| slot != local && self.mice[slot as usize].is_some())
            .map(|slot| {
                let (x, y, down) = mice[slot as usize];
                (slot, x, y, down)
            })
            .filter(|&(slot, x, y, _)| {
                as_player(player, &privates, slot, |player| {
                    let target = player.coop_click_target(x, y);
                    // not over another player's private screen, which this PC doesn't draw
                    let unseen = target.as_deref().is_some_and(|target| {
                        privates.iter().any(|p| p.owner.is_some_and(|o| o != local) && inside(target, &p.path))
                    });
                    target.is_some() && !unseen && may_click(player, mode, &zones, slot, x, y)
                }) && !privates.iter().any(|p| {
                    // nor over this PC's own private screen, which that player doesn't see
                    p.owner == Some(local) && player.coop_clip_covers(&p.path, x, y)
                })
            })
            .collect()
    }

    /// This PC's own mouse (stage twips), for this player's own hover.
    pub fn set_local_mouse(&mut self, at: Option<(i32, i32)>) {
        self.local_mouse = at;
    }

    /// `hover: own`: this PC draws Button symbols in the game's own Up/Over art by its own
    /// player's mouse, where that player may click (the picture only; every PC's game is
    /// unchanged). Hover made by the game's code still follows the host's mouse.
    fn show_own_hover(&mut self, player: &mut Player) {
        let Ok((mode, mut zones, privates, hover, local)) = self
            .api
            .lock()
            .map(|api| (api.mouse_mode, api.zones.clone(), api.privates.clone(), api.hover, api.local_slot))
        else {
            return;
        };
        zones.extend(owned_zones(&privates));
        let on = mode != MouseMode::Host && hover == Some(Hover::Own);
        let at = self.local_mouse.filter(|_| on);
        as_player(player, &privates, local, |player| {
            let at = at.filter(|&(x, y)| may_click(player, mode, &zones, local, x, y));
            player.coop_set_own_hover(on, at);
        });
    }

    /// Clicks refused since the last call because another player had that private screen open:
    /// (who clicked, whose it is).
    pub fn take_busy(&mut self) -> Vec<(u8, u8)> {
        std::mem::take(&mut self.busy)
    }

    /// Who each private screen belongs to (the same on every PC: game state and whose click came
    /// last), and whether this PC draws it: only its owner's does.
    fn update_privates(&mut self, player: &mut Player) {
        let Ok((mut privates, local)) = self.api.lock().map(|api| (api.privates.clone(), api.local_slot)) else {
            return;
        };
        if privates.is_empty() {
            return;
        }
        for private in &mut privates {
            private.owner = if player.coop_clip_shown(&private.path) {
                private.fixed.or(private.owner).or(self.last_clicker).or(Some(0))
            } else {
                None
            };
            player.coop_set_drawn(&private.path, private.owner.is_none_or(|owner| owner == local));
        }
        if let Ok(mut api) = self.api.lock() {
            for private in privates {
                if let Some(p) = api.privates.iter_mut().find(|p| p.path == private.path) {
                    p.owner = private.owner;
                }
            }
        }
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
            // the game's own map changes who may click only if it says so
            if let Some(mode) = self.keys.mouse {
                api.mouse_mode = mode;
            }
            if !self.keys.zones.is_empty() {
                api.zones = self.keys.zones.clone();
            }
            if !self.keys.privates.is_empty() {
                api.privates = privates(&self.keys.privates);
            }
            if self.keys.hover.is_some() {
                api.hover = self.keys.hover;
            }
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

/// May `slot` click at the stage point (x, y) (twips)? Never in a box that belongs to someone
/// else; the host anywhere else; a friend as the mode says: never (`host`), only in their own box
/// (`zones`) or anywhere (`buttons`), on something the game made clickable, or anywhere (`all`).
fn may_click(player: &mut Player, mode: MouseMode, zones: &[Zone], slot: u8, x: i32, y: i32) -> bool {
    let target = player.coop_click_target(x, y);
    let target = target.as_deref();
    Zone::allows(zones, slot, x, y, target)
        && match mode {
            _ if slot == 0 => true,
            MouseMode::Host => false,
            MouseMode::Zones => target.is_some() && Zone::is_own(zones, slot, x, y, target),
            MouseMode::Buttons => target.is_some(),
            MouseMode::All => true,
        }
}

/// Who may click what this frame: the mode, the zones (private screens' included) and the
/// private screens.
struct ClickRules {
    mode: MouseMode,
    zones: Vec<Zone>,
    privates: Vec<Private>,
}

/// Runs `f` (mouse events, picking) as player `slot` sees the game: other players' open private
/// screens are out of the way, so the mouse goes through them. They are put back after.
fn as_player<R>(player: &mut Player, privates: &[Private], slot: u8, f: impl FnOnce(&mut Player) -> R) -> R {
    let mut stepped_aside = vec![];
    for private in privates.iter().filter(|p| p.owner.is_some_and(|owner| owner != slot)) {
        if let Some(was) = player.coop_set_pickable(&private.path, false) {
            stepped_aside.push((private.path.as_str(), was));
        }
    }
    let result = f(player);
    for (path, was) in stepped_aside.into_iter().rev() {
        player.coop_set_pickable(path, was);
    }
    result
}

/// An open private screen is a zone of its owner's.
fn owned_zones(privates: &[Private]) -> impl Iterator<Item = Zone> + '_ {
    privates.iter().filter_map(|p| {
        Some(Zone { slot: p.owner?, area: crate::coop::keymap::ZoneArea::Clip(p.path.clone()) })
    })
}

/// The key map's private screens, nobody's yet.
fn privates(specs: &[(String, Option<String>)]) -> Vec<Private> {
    specs
        .iter()
        .map(|(path, openers)| Private { path: path.clone(), fixed: None, openers: openers.clone(), owner: None })
        .collect()
}

/// Whether `target` is the clip at `path` or inside it.
fn inside(target: &str, path: &str) -> bool {
    target == path || target.strip_prefix(path).is_some_and(|rest| rest.starts_with('.'))
}

/// Moves the game's mouse to the stage point (x, y) (twips). Inputs carry stage positions, used
/// exactly as they are on every PC (the window position is only for Ruffle's event plumbing).
fn move_mouse(player: &mut Player, x: i32, y: i32) {
    player.set_stage_mouse_override(Some((x, y)));
    let (x, y) = player.stage_to_viewport(x, y);
    player.handle_event(PlayerEvent::MouseMove { x, y });
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

    /// Ruffle's own AS2 keyPress test (on(keyPress "a") traces "keyPress a"): a button press from
    /// input alone (a script, a replay) reaches it, with a map like a game's own `coop.keys`, for
    /// a friend's slot too, and only once while held.
    #[test]
    fn a_pressed_button_reaches_keypress_buttons() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/tests/swfs");
        let movie =
            ruffle_core::tag_utils::movie_from_path(root.join("avm1/button_key_events/test.swf"), None).expect("test swf");
        let traces = Arc::new(Mutex::new(vec![]));
        let player = PlayerBuilder::new()
            .with_movie(movie)
            .with_log(Traces(traces.clone()))
            .with_autoplay(true)
            .build();
        let mut player = player.lock().expect("player");
        let keys = Game::parse("game", "slot 0: FIRE=65\nslot 1: FIRE=65").expect("map").keys();
        let mut stepper = Stepper::new(keys, 2, 0);
        let none = InputFrame::default();
        let fire = InputFrame { buttons: FIRE, ..Default::default() };
        stepper.step(&mut player, &[none, none]);
        stepper.step(&mut player, &[none, fire]);
        stepper.step(&mut player, &[none, fire]);
        let presses = traces.lock().expect("traces").iter().filter(|t| *t == "keyPress a").count();
        assert_eq!(presses, 1);
    }

    /// Friends' clicks (a test SWF: a button-mode button at 100,100 100x60 traces "button" when
    /// clicked, any press traces "down X,Y"). By default only the host clicks; `mouse: buttons`
    /// lets a friend click what the game made clickable and nothing else; `all` anywhere; a zone
    /// keeps its box to its owner. The game's mouse rests at the host's after a friend's click.
    #[test]
    fn friends_click_what_the_rules_allow() {
        let swf = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/coop/clicks/Clicks.swf");
        let run = |rules: &str, clicks: &[(usize, i32, i32)]| -> Vec<String> {
            let movie = ruffle_core::tag_utils::movie_from_path(&swf, None).expect("test swf");
            let traces = Arc::new(Mutex::new(vec![]));
            let player = PlayerBuilder::new().with_movie(movie).with_log(Traces(traces.clone())).with_autoplay(true).build();
            let mut player = player.lock().expect("player");
            let keys = Game::parse("t", &format!("slot 0: FIRE=32
slot 1: FIRE=32
{rules}")).expect("map").keys();
            let mut stepper = Stepper::new(keys, 2, 0);
            let at = |x: i32, y: i32, down: bool| Some((x * 20, y * 20, down));
            let rest = || [InputFrame { mouse: at(10, 10, false), ..Default::default() }, InputFrame::default()];
            stepper.step(&mut player, &rest());
            for &(slot, x, y) in clicks {
                for down in [true, false] {
                    let mut inputs = rest();
                    inputs[slot].mouse = at(x, y, down);
                    stepper.step(&mut player, &inputs);
                }
            }
            stepper.step(&mut player, &rest());
            let rest = if rules.contains("hover: off") { PARKED } else { (200, 200) };
            assert_eq!(player.mouse_position_twips(), rest, "where the mouse rests ({rules})");
            traces.lock().expect("traces").clone()
        };
        let on_button = [(1, 150, 130)];
        let off_button = [(1, 300, 250)];
        assert_eq!(run("", &on_button), Vec::<String>::new(), "only the host clicks by default");
        assert_eq!(run("", &[(0, 150, 130)]), ["down 150,130", "button"]);
        assert_eq!(run("mouse: buttons", &on_button), ["down 150,130", "button"]);
        assert_eq!(run("mouse: buttons\nhover: host", &on_button), ["down 150,130", "button"], "hover at the host's");
        assert_eq!(run("mouse: buttons\nhover: off", &on_button), ["down 150,130", "button"], "no hover: parked");
        assert_eq!(run("mouse: buttons", &off_button), Vec::<String>::new(), "not on something clickable");
        assert_eq!(run("mouse: all", &off_button), ["down 300,250"]);
        let zone = "mouse: buttons
zone 0: 90,90 130x80";
        assert_eq!(run(zone, &on_button), Vec::<String>::new(), "the host's box");
        assert_eq!(run(zone, &[(0, 150, 130)]), ["down 150,130", "button"]);
        assert_eq!(run("mouse: buttons", &[(1, 150, 130), (0, 300, 250)]), ["down 150,130", "button", "down 300,250"]);
        // `zones`: a friend only in their own box, the host everywhere else
        assert_eq!(run("mouse: zones", &on_button), Vec::<String>::new(), "no box of their own");
        let own = "mouse: zones
zone 1: 90,90 130x80";
        assert_eq!(run(own, &on_button), ["down 150,130", "button"]);
        assert_eq!(run(own, &[(0, 150, 130)]), Vec::<String>::new(), "the friend's box");
        assert_eq!(run(own, &[(0, 300, 250)]), ["down 300,250"]);
        // a friend's click that never lets go (released outside their window) doesn't lock the
        // host out: it is let go where the host clicks (here the same button, which opens the test
        // SWF's pop-up over itself), then the host's click goes through (onto that pop-up)
        let stuck = |rules: &str| {
            let movie = ruffle_core::tag_utils::movie_from_path(&swf, None).expect("test swf");
            let traces = Arc::new(Mutex::new(vec![]));
            let player = PlayerBuilder::new().with_movie(movie).with_log(Traces(traces.clone())).with_autoplay(true).build();
            let mut player = player.lock().expect("player");
            let keys = Game::parse("t", &format!("slot 0: FIRE=32
slot 1: FIRE=32
{rules}")).expect("map").keys();
            let mut stepper = Stepper::new(keys, 2, 0);
            let at = |x: i32, y: i32, down: bool| Some((x * 20, y * 20, down));
            let frame = |host, friend| [InputFrame { mouse: host, ..Default::default() }, InputFrame { mouse: friend, ..Default::default() }];
            stepper.step(&mut player, &frame(at(10, 10, false), None));
            stepper.step(&mut player, &frame(at(10, 10, false), at(150, 130, true))); // the friend presses, forever
            for _ in 0..3 {
                stepper.step(&mut player, &frame(at(10, 10, false), at(150, 130, true)));
            }
            stepper.step(&mut player, &frame(at(150, 130, true), at(150, 130, true)));
            stepper.step(&mut player, &frame(at(150, 130, false), at(150, 130, true)));
            traces.lock().expect("traces").clone()
        };
        assert_eq!(stuck("mouse: buttons"), ["down 150,130", "button", "down 150,130"], "the host still clicks");
    }

    /// A private screen (the test SWF's pop-up, opened by its button): it belongs to the friend
    /// whose click opened it, the host's click goes through it to the button under it, only the
    /// owner's PC draws it, and its owner can use it; once closed it is nobody's.
    #[test]
    fn a_private_screen_belongs_to_whoever_opened_it() {
        let swf = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/coop/clicks/Clicks.swf");
        for local in [0u8, 1] {
            let movie = ruffle_core::tag_utils::movie_from_path(&swf, None).expect("test swf");
            let traces = Arc::new(Mutex::new(vec![]));
            let player = PlayerBuilder::new().with_movie(movie).with_log(Traces(traces.clone())).with_autoplay(true).build();
            let mut player = player.lock().expect("player");
            let keys = Game::parse("t", "slot 0: FIRE=32
slot 1: FIRE=32
mouse: buttons
private: _root.popup")
                .expect("map")
                .keys();
            let mut stepper = Stepper::new(keys, 2, local);
            let at = |x: i32, y: i32, down: bool| Some((x * 20, y * 20, down));
            let click = |stepper: &mut Stepper, player: &mut Player, slot: usize, x: i32, y: i32| {
                for down in [true, false] {
                    let mut inputs = [InputFrame { mouse: at(10, 10, false), ..Default::default() }, InputFrame::default()];
                    inputs[slot].mouse = at(x, y, down);
                    stepper.step(player, &inputs);
                }
            };
            let owner = |stepper: &Stepper| stepper.api().lock().expect("api").privates[0].owner;
            stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
            click(&mut stepper, &mut player, 1, 150, 130); // the friend opens the pop-up
            assert_eq!(owner(&stepper), Some(1));
            assert_eq!(player.coop_set_drawn("_root.popup", true), true, "the pop-up exists");
            stepper.update_privates(&mut player);
            click(&mut stepper, &mut player, 0, 150, 130); // the host clicks through it, onto the button
            assert_eq!(owner(&stepper), Some(1));
            click(&mut stepper, &mut player, 0, 250, 185); // the host can't use the pop-up's button
            click(&mut stepper, &mut player, 1, 250, 185); // its owner can
            assert_eq!(owner(&stepper), None, "closed: nobody's");
            let traces = traces.lock().expect("traces").clone();
            assert_eq!(
                traces,
                ["down 150,130", "button", "down 150,130", "button", "down 250,185", "down 250,185", "popup"],
                "PC of player {}",
                local + 1
            );
        }
    }

    /// Four players on the click test SWF (a button at 100,100 100x60 that opens a pop-up over
    /// itself; any press traces "down X,Y"). Clicks that land in the same frame all reach the game,
    /// one after another in slot order; a friend's zone keeps out another friend; a friend holding
    /// the button gives way to waiting clicks; a private pop-up is its opener's with four players.
    #[test]
    fn four_players_take_turns_on_the_one_mouse() {
        let swf = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/coop/clicks/Clicks.swf");
        type Mice = [Option<(i32, i32, bool)>; 4];
        let run = |rules: &str, frames: &[Mice]| -> (Vec<String>, Stepper) {
            let movie = ruffle_core::tag_utils::movie_from_path(&swf, None).expect("test swf");
            let traces = Arc::new(Mutex::new(vec![]));
            let player = PlayerBuilder::new().with_movie(movie).with_log(Traces(traces.clone())).with_autoplay(true).build();
            let mut player = player.lock().expect("player");
            let map = format!("slot 0: FIRE=32\nslot 1: FIRE=32\nslot 2: FIRE=32\nslot 3: FIRE=32\n{rules}");
            let mut stepper = Stepper::new(Game::parse("t", &map).expect("map").keys(), 4, 0);
            let rest = |mice: &Mice| -> [InputFrame; 4] {
                std::array::from_fn(|slot| InputFrame { mouse: mice[slot], ..Default::default() })
            };
            let idle: Mice = [Some((200, 200, false)), None, None, None];
            stepper.step(&mut player, &rest(&idle));
            for mice in frames {
                stepper.step(&mut player, &rest(mice));
            }
            for _ in 0..3 {
                stepper.step(&mut player, &rest(frames.last().unwrap_or(&idle)));
            }
            let traces = traces.lock().expect("traces").clone();
            (traces, stepper)
        };
        let at = |x: i32, y: i32, down: bool| Some((x * 20, y * 20, down));
        let host = Some((200, 200, false));

        // three friends click in the same frame: all three reach the game, in slot order
        let together = [
            [host, at(300, 250, true), at(310, 250, true), at(320, 250, true)],
            [host, at(300, 250, false), at(310, 250, false), at(320, 250, false)],
        ];
        let (traces, _) = run("mouse: all", &together);
        assert_eq!(traces, ["down 300,250", "down 310,250", "down 320,250"]);

        // player 3's box keeps out player 4 (and the host); player 3 clicks it
        let boxed = [
            [host, None, at(300, 250, true), at(305, 250, true)],
            [host, None, at(300, 250, false), at(305, 250, false)],
        ];
        let (traces, _) = run("mouse: all\nzone 2: 280,230 60x40", &boxed);
        assert_eq!(traces, ["down 300,250"], "player 4 may not click in player 3's box");

        // player 2 holds the button down; player 3's click waits, then gets its turn
        let mut held: Vec<Mice> = vec![[host, at(300, 250, true), at(310, 250, true), None]];
        held.extend(std::iter::repeat_n([host, at(300, 250, true), at(310, 250, false), None], 40));
        held.push([host, at(300, 250, false), at(310, 250, false), None]);
        let (traces, _) = run("mouse: all", &held);
        assert_eq!(traces, ["down 300,250", "down 310,250"], "a long hold gives way");

        // player 3 opens the pop-up: it is theirs; player 2 clicks through it onto the button;
        // player 4 can't use the pop-up's own button; its owner can
        let private = [
            [host, None, at(150, 130, true), None],
            [host, None, at(150, 130, false), None],
            [host, at(150, 130, true), None, at(250, 185, true)],
            [host, at(150, 130, false), None, at(250, 185, false)],
            [host, None, at(250, 185, true), None],
            [host, None, at(250, 185, false), None],
        ];
        let (traces, stepper) = run("mouse: buttons\nprivate: _root.popup", &private);
        assert_eq!(traces, ["down 150,130", "button", "down 150,130", "button", "down 250,185", "popup"]);
        assert_eq!(stepper.api().lock().expect("api").privates[0].owner, None, "closed by its owner");
    }

    /// A game's own `coop.keys` map without `mouse:` or `zone` lines keeps the click rules the
    /// game set (the API demo sets `coop.mouse("buttons")`, then its key map).
    #[test]
    fn a_game_key_map_keeps_the_click_rules() {
        let swf = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/coop/clicks/Clicks.swf");
        let movie = ruffle_core::tag_utils::movie_from_path(&swf, None).expect("test swf");
        let player = PlayerBuilder::new().with_movie(movie).with_autoplay(true).build();
        let mut player = player.lock().expect("player");
        let keys = Game::parse("t", "slot 0: FIRE=32
slot 1: FIRE=32").expect("map").keys();
        let mut stepper = Stepper::new(keys, 2, 0);
        stepper.step(&mut player, &[InputFrame::default(), InputFrame::default()]);
        {
            let mut api = stepper.api().lock().expect("api");
            api.answer("coop.mouse", &[Value::String("buttons".into())]);
            api.answer("coop.zone", &[1.0, 1.0, 2.0, 3.0, 4.0].map(Value::Number));
            api.answer("coop.keys", &[Value::String("slot 0: FIRE=13
slot 1: FIRE=13".into())]);
        }
        stepper.apply_game_calls(&mut player);
        let api = stepper.api().lock().expect("api");
        assert_eq!((api.mouse_mode, api.zones.len()), (MouseMode::Buttons, 1));
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
