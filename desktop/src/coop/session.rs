//! A Ruffle Together session: the handshake, then every frame in lockstep with the other
//! players, a sync check every 25 frames, and what happens when someone leaves.

use crate::coop::frame::Stepper;
use crate::coop::keymap::{GameKeys, local_buttons};
use crate::coop::lockstep::{InputFrame, Lockstep};
use crate::coop::net::{Event, Link, Role, connect};
use crate::coop::protocol::Msg;
use crate::coop::script::Script;
use crate::coop::storage::SessionStorage;
use anyhow::{Result, anyhow, bail};
use ruffle_core::Player;
use ruffle_core::events::PhysicalKey;
use ruffle_core::frame_clock::FrameClock;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// v1 plays 2 PCs (the code is ready for up to 4).
const PLAYERS: u8 = 2;
/// The sync check runs every this many frames.
const CHECK_EVERY: u32 = 25;
/// Most frames run in one tick when catching up after a stall.
const MAX_CATCH_UP: u32 = 10;
/// "Waiting for <name>" shows after this long without a frame.
const SHOW_WAITING: Duration = Duration::from_secs(2);
/// A player this long silent while the game waits is treated as gone.
const GIVE_UP: Duration = Duration::from_secs(5);

/// This build, as the relay and the other players see it: a different build is refused.
pub fn build_id() -> String {
    format!("rc-{}#{:.10}", env!("CARGO_PKG_VERSION"), env!("VERGEN_GIT_SHA"))
}

pub struct SessionOpts {
    pub role: Role,
    pub name: String,
    /// Input delay in frames (host only; the friend uses the host's).
    pub delay: u32,
    pub swf: PathBuf,
    /// Ruffle's save folder: the host shares its saves for this game.
    pub save_dir: Option<PathBuf>,
    /// Testing: input from a script instead of the keyboard and mouse.
    pub script: Option<Script>,
    /// How long the host waits for a friend (None: until the window closes).
    pub host_wait: Option<Duration>,
    /// Testing: shift this PC's start date, to force a desync.
    pub start_skew_ms: i64,
    /// Run at the game's frame rate (always, except in fast tests).
    pub paced: bool,
    /// Player settings that change how a game behaves, as "name=value; ..." (see
    /// `settings_line`); every player must have the same.
    pub settings: String,
    /// Show the room code in a window (there is no game window yet while the host waits).
    pub dialogs: bool,
}

/// The player settings that change how a game behaves, for the handshake.
pub fn settings_line(launch: &crate::player::LaunchOptions) -> String {
    let p = &launch.player;
    [
        format!("flashvars={:?}", p.parameters),
        format!("quality={:?}", p.quality),
        format!("align={:?}", p.align),
        format!("force_align={:?}", p.force_align),
        format!("scale={:?}", p.scale),
        format!("force_scale={:?}", p.force_scale),
        format!("load_behavior={:?}", p.load_behavior),
        format!("letterbox={:?}", p.letterbox),
        format!("player_version={:?}", p.player_version),
        format!("player_runtime={:?}", p.player_runtime),
        format!("frame_rate={:?}", p.frame_rate),
        format!("dummy_external_interface={:?}", p.dummy_external_interface),
        format!("spoof_url={:?}", p.spoof_url.as_ref().map(|u| u.as_str())),
        format!("avm2_optimizer={:?}", launch.avm2_optimizer_enabled),
    ]
    .join("; ")
}

/// The first setting that differs, e.g. "frame_rate (Some(60.0) here, None on the other PC)".
fn settings_difference(mine: &str, theirs: &str) -> Option<String> {
    let theirs: HashMap<&str, &str> =
        theirs.split("; ").filter_map(|kv| kv.split_once('=')).collect();
    mine.split("; ").filter_map(|kv| kv.split_once('=')).find_map(|(name, value)| {
        let other = theirs.get(name).copied().unwrap_or("missing");
        (other != value).then(|| format!("{name} ({value} here, {other} on the other PC)"))
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionState {
    pub waiting_on: Vec<String>,
    pub desync_at: Option<u32>,
    pub closed: Option<String>,
}

pub struct Session {
    link: Link,
    lockstep: Lockstep,
    stepper: Stepper,
    local_index: u8,
    names: Vec<String>,
    /// Connection number -> player slot of the session's players (see `from` in net events).
    peers: HashMap<u8, u8>,
    hosting: bool,
    room: Option<String>,
    clock: Arc<Mutex<FrameClock>>,
    storage: Option<SessionStorage>,
    script: Option<Script>,
    /// How many local inputs were sent (the next one is for frame `delay + local_sent`).
    local_sent: u32,
    held: HashSet<PhysicalKey>,
    focused: bool,
    mouse: Option<(i32, i32, bool)>,
    my_prints: HashMap<u32, u64>,
    peer_prints: HashMap<(u8, u32), u64>,
    /// Readable game state at recent check frames, written out when a desync shows up.
    my_states: HashMap<u32, String>,
    last_frame_at: Instant,
    paced: bool,
    /// When the next frame is due (paced sessions).
    next_due: Option<Instant>,
    /// When each local input was taken, by the frame it is for: the time until that frame
    /// runs is the input delay this player feels.
    taken_at: HashMap<u32, Instant>,
    felt_delay: Vec<Duration>,
    state: SessionState,
}

fn hash_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path).map_err(|e| anyhow!("can't read {}: {e}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(data)))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// The next event, waiting up to `timeout`.
fn next_event(link: &Link, timeout: Duration) -> Option<Event> {
    let end = Instant::now() + timeout;
    loop {
        if let Some(event) = link.try_recv() {
            return Some(event);
        }
        if Instant::now() >= end {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

impl Session {
    /// Connects and does the handshake; returns once the game can start.
    pub fn start(opts: SessionOpts) -> Result<Session> {
        let build = build_id();
        let swf_sha256 = hash_file(&opts.swf)?;
        let hosting = matches!(opts.role, Role::HostDirect { .. } | Role::HostRelay { .. });
        let link = connect(opts.role, &build, &opts.name)?;
        let room = link.room_code();
        if let Some(code) = &room {
            tracing::info!("Ruffle Together: room code {code}");
            println!("Ruffle Together room code: {code}");
            if opts.dialogs {
                let text = format!("Your room code is {code}\n\nTell your friend; the game starts when they join.");
                std::thread::spawn(move || {
                    let _ = rfd::MessageDialog::new()
                        .set_title("Ruffle Together")
                        .set_description(text)
                        .show();
                });
            }
        }
        let settings = opts.settings.replace('|', " ");
        let me = Msg::Hello {
            build: build.clone(),
            swf_sha256: swf_sha256.clone(),
            settings: settings.clone(),
            name: opts.name.clone(),
        };
        let mismatch = |their_build: &str, their_swf: &str, their_settings: &str| {
            if their_build != build {
                Some(format!("different Ruffle Together build ({their_build}, this one is {build})"))
            } else if their_swf != swf_sha256 {
                Some("different game file".to_string())
            } else {
                settings_difference(&settings, their_settings)
                    .map(|what| format!("different player settings: {what}"))
            }
        };

        let start = if hosting {
            let deadline = opts.host_wait.map(|wait| Instant::now() + wait);
            loop {
                let left = deadline.map_or(Duration::from_secs(3600), |d| {
                    d.saturating_duration_since(Instant::now())
                });
                if left.is_zero() {
                    bail!("no friend joined");
                }
                match next_event(&link, left.min(Duration::from_millis(500))) {
                    Some(Event::Msg {
                        from,
                        msg: Msg::Hello { build: b, swf_sha256: s, settings: o, name },
                    }) => {
                        if let Some(reason) = mismatch(&b, &s, &o) {
                            tracing::warn!("Ruffle Together: refused {name}: {reason}");
                            link.send(&Msg::Refuse { reason });
                            continue;
                        }
                        let save = match &opts.save_dir {
                            Some(dir) => SessionStorage::export_from_disk(
                                dir,
                                &opts.swf.file_name().unwrap_or_default().to_string_lossy(),
                            ),
                            None => vec![],
                        };
                        let start = Msg::Start {
                            player_count: PLAYERS,
                            // the friend's player slot (not its connection number: a refused
                            // or stray connection must not shift it)
                            your_index: 1,
                            delay: opts.delay,
                            start_unix_ms: now_unix_ms(),
                            frame_rate_milli: 0,
                            save,
                        };
                        // the game starts: only this friend stays connected, and nobody new
                        link.keep_only(&[from]);
                        link.stop_accepting();
                        link.send(&me);
                        link.send(&start);
                        break (start, vec![opts.name.clone(), name], from);
                    }
                    Some(Event::Closed(reason)) => bail!("{reason}"),
                    _ => {}
                }
            }
        } else {
            link.send(&me);
            let mut host_name = String::new();
            loop {
                match next_event(&link, Duration::from_secs(30)) {
                    Some(Event::Msg { msg: Msg::Refuse { reason }, .. }) => bail!("{reason}"),
                    Some(Event::Msg {
                        msg: Msg::Hello { build: b, swf_sha256: s, settings: o, name },
                        ..
                    }) => {
                        if let Some(reason) = mismatch(&b, &s, &o) {
                            bail!("{reason}");
                        }
                        host_name = name;
                    }
                    Some(Event::Msg { msg: start @ Msg::Start { .. }, .. }) => {
                        break (start, vec![host_name, opts.name.clone()], 0);
                    }
                    Some(Event::Closed(reason)) => bail!("{reason}"),
                    Some(_) => {}
                    None => bail!("the host didn't answer"),
                }
            }
        };

        let (Msg::Start { player_count, your_index, delay, start_unix_ms, save, .. }, names, peer) =
            start
        else {
            unreachable!("start is always a Start message");
        };
        let storage = SessionStorage::from_bytes(&save).ok_or_else(|| anyhow!("bad save data"))?;
        // the Start message names the friend's slot; the host is always player 0
        let your_index = if hosting { 0 } else { your_index };
        tracing::info!(
            "Ruffle Together: playing as player {} of {player_count}, input delay {delay} frames",
            your_index + 1
        );
        Ok(Session {
            link,
            lockstep: Lockstep::new(player_count, your_index, delay),
            stepper: Stepper::new(GameKeys::bad_ice_cream()),
            local_index: your_index,
            // the host knows its friend by connection number; a friend hears everything from the host
            peers: HashMap::from([(peer, if hosting { 1 } else { 0 })]),
            hosting,
            names,
            room,
            clock: Arc::new(Mutex::new(FrameClock::new(start_unix_ms + opts.start_skew_ms))),
            storage: Some(storage),
            script: opts.script,
            local_sent: 0,
            held: HashSet::new(),
            focused: true,
            mouse: None,
            my_prints: HashMap::new(),
            peer_prints: HashMap::new(),
            my_states: HashMap::new(),
            last_frame_at: Instant::now(),
            paced: opts.paced,
            next_due: None,
            taken_at: HashMap::new(),
            felt_delay: vec![],
            state: SessionState::default(),
        })
    }

    /// The clock the player must use (give it to `set_frame_clock` and a `FrameClockLocale`).
    pub fn clock(&self) -> Arc<Mutex<FrameClock>> {
        self.clock.clone()
    }

    /// The save data the player must use for this session (once).
    pub fn take_storage(&mut self) -> SessionStorage {
        self.storage.take().unwrap_or_default()
    }

    /// The next frame to run.
    #[cfg(test)]
    pub fn frame(&self) -> u32 {
        self.lockstep.frame()
    }

    #[cfg(test)]
    pub fn print_at(&self, frame: u32) -> Option<u64> {
        self.my_prints.get(&frame).copied()
    }

    pub fn key(&mut self, key: PhysicalKey, down: bool) {
        if down {
            self.held.insert(key);
        } else {
            self.held.remove(&key);
        }
    }

    /// The host's mouse, in stage twips (see `Player::viewport_to_stage`).
    pub fn mouse(&mut self, x: i32, y: i32, down: Option<bool>) {
        let was_down = self.mouse.is_some_and(|(_, _, down)| down);
        self.mouse = Some((x, y, down.unwrap_or(was_down)));
    }

    /// Keys held while the window loses focus would stay pressed; let go of them.
    pub fn focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.held.clear();
        }
    }

    fn local_input(&self) -> InputFrame {
        if let Some(script) = &self.script {
            return script.input(self.local_sent as usize, self.local_index as usize);
        }
        InputFrame {
            buttons: if self.focused { local_buttons(&self.held) } else { 0 },
            mouse: if self.local_index == 0 { self.mouse } else { None },
        }
    }

    /// Runs every frame that can run now. Call often (every event-loop pass).
    pub fn tick(&mut self, player: &mut Player) -> SessionState {
        if self.state.closed.is_some() {
            return self.state.clone();
        }
        while let Some(event) = self.link.try_recv() {
            match event {
                Event::Msg { from, msg } => {
                    // only the session's players count, and each only for its own slot (the
                    // host passes the other friends' messages on, so a friend trusts the host)
                    let Some(&slot) = self.peers.get(&from) else {
                        continue;
                    };
                    let player_of = |p: u8| !self.hosting || p == slot;
                    match msg {
                        Msg::Input { player: p, frame, input }
                            if p != self.local_index && player_of(p) =>
                        {
                            self.lockstep.push_remote(p, frame, input);
                        }
                        Msg::Print { player: p, frame, hash }
                            if p != self.local_index && player_of(p) =>
                        {
                            self.peer_prints.insert((p, frame), hash);
                            self.compare(frame);
                        }
                        Msg::Bye { player: p, reason } if player_of(p) => {
                            self.close(format!("{} left ({reason})", self.name(p)));
                        }
                        _ => {}
                    }
                }
                Event::Left { player: conn } => {
                    if let Some(&slot) = self.peers.get(&conn) {
                        self.close(format!("{} left", self.name(slot)));
                    }
                }
                Event::Closed(reason) => self.close(reason),
                _ => {}
            }
        }
        if self.state.closed.is_some() {
            self.stepper.release_all(player);
            return self.state.clone();
        }

        let now = Instant::now();
        let frame_time = Duration::from_secs_f64(1.0 / player.frame_rate());
        for _ in 0..MAX_CATCH_UP {
            // paced: a frame runs when it is due by this PC's clock; a PC that fell behind
            // (waiting for a late input) runs several in a row to catch up
            let due = *self.next_due.get_or_insert(now);
            if self.paced && now < due {
                break;
            }
            // this PC's input for `delay` frames ahead
            while self.local_sent <= self.lockstep.frame() {
                let input = self.local_input();
                let (frame, input) = self.lockstep.push_local(input);
                self.taken_at.insert(frame, now);
                self.link.send(&Msg::Input { player: self.local_index, frame, input });
                self.local_sent += 1;
            }
            let Some(inputs) = self.lockstep.ready() else {
                break;
            };
            self.stepper.step(player, &inputs);
            // after a long stall, don't sprint through more than the catch-up allows
            self.next_due = Some((due + frame_time).max(now - frame_time * MAX_CATCH_UP));
            let frame = self.lockstep.frame();
            if let Some(taken) = self.taken_at.remove(&frame) {
                self.felt_delay.push(taken.elapsed());
            }
            self.lockstep.advance();
            self.last_frame_at = Instant::now();
            if frame % CHECK_EVERY == CHECK_EVERY - 1 {
                let hash = player.coop_fingerprint();
                self.my_prints.insert(frame, hash);
                self.my_states.insert(frame, player.coop_describe());
                self.my_states.retain(|f, _| f + CHECK_EVERY * 8 > frame);
                self.link.send(&Msg::Print { player: self.local_index, frame, hash });
                self.compare(frame);
            }
        }

        let stalled = self.last_frame_at.elapsed();
        self.state.waiting_on = if stalled > SHOW_WAITING {
            self.lockstep.waiting_on().into_iter().map(|p| self.name(p)).collect()
        } else {
            vec![]
        };
        if stalled > GIVE_UP && !self.state.waiting_on.is_empty() {
            let who = self.state.waiting_on.join(", ");
            self.close(format!("{who} stopped answering"));
            self.stepper.release_all(player);
        }
        self.state.clone()
    }

    fn name(&self, player: u8) -> String {
        self.names
            .get(player as usize)
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Player {}", player + 1))
    }

    fn close(&mut self, reason: String) {
        if self.state.closed.is_none() {
            tracing::warn!("Ruffle Together: {reason}");
            self.state.closed = Some(reason);
        }
    }

    /// The first frame where the PCs' fingerprints differ is reported once, on every PC.
    fn compare(&mut self, frame: u32) {
        let Some(&mine) = self.my_prints.get(&frame) else {
            return;
        };
        let differs = self
            .peer_prints
            .iter()
            .any(|(&(_, f), &theirs)| f == frame && theirs != mine);
        if differs && self.state.desync_at.is_none_or(|at| frame < at) {
            self.state.desync_at = Some(frame);
            tracing::error!("Ruffle Together: the games are out of sync at frame {frame}");
            self.write_desync_report(frame);
        }
    }

    fn write_desync_report(&self, frame: u32) {
        let mut text = format!(
            "Ruffle Together desync at frame {frame}\nthis PC: player {}\n",
            self.local_index + 1
        );
        if let Some(mine) = self.my_prints.get(&frame) {
            text.push_str(&format!("this PC's fingerprint: {mine:016x}\n"));
        }
        for ((p, f), theirs) in &self.peer_prints {
            if *f == frame {
                text.push_str(&format!("{}'s fingerprint: {theirs:016x}\n", self.name(*p)));
            }
        }
        if let Some(state) = self.my_states.get(&frame) {
            text.push_str("\nthis PC's game state:\n");
            text.push_str(state);
        }
        let path = std::env::temp_dir().join(format!(
            "ruffle-together-desync-{}-p{}.txt",
            now_unix_ms(),
            self.local_index + 1
        ));
        match std::fs::write(&path, text) {
            Ok(()) => tracing::error!("Ruffle Together: desync report written to {}", path.display()),
            Err(e) => tracing::warn!("Ruffle Together: can't write the desync report: {e}"),
        }
    }

    /// How long this PC's inputs took to reach the game (input taken -> its frame runs).
    pub fn felt_delay(&self) -> &[Duration] {
        &self.felt_delay
    }

    /// The next frame to run (for logs).
    pub fn frames_run(&self) -> u32 {
        self.lockstep.frame()
    }

    /// What the window title shows.
    pub fn title(&self) -> String {
        let mut title = String::from("Ruffle Together");
        if let Some(code) = &self.room {
            title.push_str(&format!(" - room {code}"));
        }
        title.push_str(&format!(" - player {}", self.local_index + 1));
        if let Some(reason) = &self.state.closed {
            title.push_str(&format!(" - {reason}"));
        } else if self.state.desync_at.is_some() {
            title.push_str(" - THE TWO GAMES ARE OUT OF SYNC: restart the level");
        } else if !self.state.waiting_on.is_empty() {
            title.push_str(&format!(" - waiting for {}...", self.state.waiting_on.join(", ")));
        }
        title
    }
}

/// `--coop-headless`: plays `frames` frames without a window and prints one result line.
pub fn run_headless(mut session: Session, swf: &Path, frames: u32) -> Result<()> {
    let player =
        crate::coop::replay::headless_player(swf, Some(session.clock()), session.take_storage())?;
    let mut player = player.lock().map_err(|_| anyhow!("player lock"))?;
    let started = Instant::now();
    let state = loop {
        let state = session.tick(&mut player);
        if session.frames_run() >= frames || state.closed.is_some() {
            break state;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    let mut delays: Vec<f64> = session.felt_delay().iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    delays.sort_by(f64::total_cmp);
    let at = |q: f64| delays.get(((delays.len() as f64 - 1.0) * q) as usize).copied().unwrap_or(-1.0);
    println!(
        "COOP RESULT player={} frames={} seconds={:.1} desync_at={} closed={} \
         felt_delay_ms_median={:.0} p95={:.0} max={:.0}",
        session.local_index + 1,
        session.frames_run(),
        started.elapsed().as_secs_f64(),
        state.desync_at.map_or("none".into(), |f| f.to_string()),
        state.closed.as_deref().unwrap_or("no"),
        at(0.5),
        at(0.95),
        at(1.0),
    );
    Ok(())
}

impl Drop for Session {
    fn drop(&mut self) {
        self.link.send(&Msg::Bye {
            player: self.local_index,
            reason: "closed the game".into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coop::replay::headless_player;
    use std::net::TcpListener;
    use std::thread;

    fn free_port() -> u16 {
        const RESERVED: [u16; 8] = [6420, 7777, 7778, 7779, 27015, 27016, 27020, 27021];
        loop {
            let port = TcpListener::bind("127.0.0.1:0")
                .and_then(|l| l.local_addr())
                .map(|a| a.port())
                .expect("free port");
            if !RESERVED.contains(&port) {
                return port;
            }
        }
    }

    fn test_swf() -> Option<PathBuf> {
        match std::env::var("COOP_TEST_SWF") {
            Ok(path) => Some(PathBuf::from(path)),
            Err(_) => {
                eprintln!("skipping: COOP_TEST_SWF not set");
                None
            }
        }
    }

    fn script() -> Script {
        Script::parse(include_str!("../../tests/coop/bic-two-players.txt")).expect("script")
    }

    fn opts(role: Role, swf: &Path, start_skew_ms: i64) -> SessionOpts {
        SessionOpts {
            role,
            name: "tester".into(),
            delay: 3,
            swf: swf.to_path_buf(),
            save_dir: None,
            script: Some(script()),
            host_wait: Some(Duration::from_secs(20)),
            start_skew_ms,
            paced: false,
            settings: "frame_rate=None; player_version=None".into(),
            dialogs: false,
        }
    }

    /// Runs one PC: starts the session, then plays until `frames` (or until `stop_at`, when it
    /// drops the session to leave). Returns the session's final state and its fingerprints.
    fn play(opts: SessionOpts, frames: u32, stop_at: Option<u32>) -> (SessionState, Session) {
        let swf = opts.swf.clone();
        let mut session = Session::start(opts).expect("session");
        let player = headless_player(&swf, Some(session.clock()), session.take_storage())
            .expect("player");
        let mut player = player.lock().expect("player");
        let end = Instant::now() + Duration::from_secs(120);
        loop {
            let state = session.tick(&mut player);
            if session.frame() >= frames || state.closed.is_some() || Instant::now() > end {
                return (state, session);
            }
            if stop_at.is_some_and(|stop| session.frame() >= stop) {
                return (state, session);
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn two_sessions_stay_in_sync() {
        let Some(swf) = test_swf() else { return };
        let port = free_port();
        let host_swf = swf.clone();
        let host = thread::spawn(move || play(opts(Role::HostDirect { port }, &host_swf, 0), 1000, None));
        thread::sleep(Duration::from_millis(300));
        let join = thread::spawn(move || {
            let addr = format!("127.0.0.1:{port}");
            play(opts(Role::JoinDirect { addr }, &swf, 0), 1000, None)
        });
        let (host_state, host_session) = host.join().expect("host");
        let (join_state, join_session) = join.join().expect("join");
        assert_eq!(host_state.desync_at, None);
        assert_eq!(join_state.desync_at, None);
        assert!(
            host_session.frame() >= 1000 && join_session.frame() >= 1000,
            "host at {} {:?}, friend at {} {:?}",
            host_session.frame(),
            host_state,
            join_session.frame(),
            join_state
        );
        // the same game on both PCs, checked every 25 frames (frames 24, 49, ... 974 are check frames)
        assert_eq!(host_session.print_at(974), join_session.print_at(974));
        assert!(host_session.print_at(974).is_some());
    }

    #[test]
    fn paced_sessions_run_at_the_games_speed() {
        let Some(swf) = test_swf() else { return };
        let port = free_port();
        let host_swf = swf.clone();
        let paced = |mut o: SessionOpts| {
            o.paced = true;
            o
        };
        let started = Instant::now();
        let host = thread::spawn(move || {
            play(paced(opts(Role::HostDirect { port }, &host_swf, 0)), 100, None)
        });
        thread::sleep(Duration::from_millis(300));
        let join = thread::spawn(move || {
            let addr = format!("127.0.0.1:{port}");
            play(paced(opts(Role::JoinDirect { addr }, &swf, 0)), 100, None)
        });
        let (host_state, _) = host.join().expect("host");
        join.join().expect("join");
        assert_eq!(host_state.desync_at, None);
        // Bad Ice-Cream runs at 25 fps: 100 frames take 4 s (plus the handshake)
        let took = started.elapsed();
        assert!(
            took >= Duration::from_millis(3800) && took < Duration::from_secs(7),
            "100 frames took {took:?}"
        );
    }

    #[test]
    fn desync_is_reported_on_both_pcs_at_the_same_frame() {
        let Some(swf) = test_swf() else { return };
        let port = free_port();
        let host_swf = swf.clone();
        let host = thread::spawn(move || play(opts(Role::HostDirect { port }, &host_swf, 0), 1000, None));
        thread::sleep(Duration::from_millis(300));
        // the friend's clock is 1 ms off: its Math.random seed differs, as if nothing were synced
        let join = thread::spawn(move || {
            let addr = format!("127.0.0.1:{port}");
            play(opts(Role::JoinDirect { addr }, &swf, 1), 1000, None)
        });
        let (host_state, _) = host.join().expect("host");
        let (join_state, _) = join.join().expect("join");
        assert!(host_state.desync_at.is_some(), "host saw no desync");
        assert_eq!(host_state.desync_at, join_state.desync_at);
    }

    #[test]
    fn friend_leaving_stops_the_host_waiting() {
        let Some(swf) = test_swf() else { return };
        let port = free_port();
        let host_swf = swf.clone();
        let host = thread::spawn(move || play(opts(Role::HostDirect { port }, &host_swf, 0), 5000, None));
        thread::sleep(Duration::from_millis(300));
        let join = thread::spawn(move || {
            let addr = format!("127.0.0.1:{port}");
            play(opts(Role::JoinDirect { addr }, &swf, 0), 5000, Some(300))
        });
        let (_, join_session) = join.join().expect("join");
        let left_at = Instant::now();
        drop(join_session);
        let (host_state, _) = host.join().expect("host");
        assert!(host_state.closed.is_some(), "host still waiting");
        assert!(left_at.elapsed() < Duration::from_secs(6));
    }

    #[test]
    fn stray_connection_during_the_game_is_ignored() {
        let Some(swf) = test_swf() else { return };
        let port = free_port();
        let host_swf = swf.clone();
        // paced (30 fps): 150 frames take 5 s, so the stray arrives mid-game
        let paced = |mut o: SessionOpts| {
            o.paced = true;
            o
        };
        let host = thread::spawn(move || {
            play(paced(opts(Role::HostDirect { port }, &host_swf, 0)), 150, None)
        });
        thread::sleep(Duration::from_millis(300));
        let join = thread::spawn(move || {
            let addr = format!("127.0.0.1:{port}");
            play(paced(opts(Role::JoinDirect { addr }, &swf, 0)), 150, None)
        });
        // a port scanner (or anyone) connects and leaves while the game runs
        thread::sleep(Duration::from_millis(2000));
        if let Ok(mut stray) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            use std::io::Write;
            let _ = stray.write_all(b"IN|1|200|31|-|-|-\0");
        }
        let (host_state, host_session) = host.join().expect("host");
        let (join_state, _) = join.join().expect("join");
        assert_eq!(host_state.closed, None, "a stray connection ended the session");
        assert_eq!(host_state.desync_at, None);
        assert_eq!(join_state.desync_at, None);
        assert!(host_session.frame() >= 150);
    }

    #[test]
    fn refused_friend_can_retry_with_the_right_file() {
        let Some(swf) = test_swf() else { return };
        let dir = tempfile::tempdir().expect("temp dir");
        let wrong = dir.path().join("wrong.swf");
        std::fs::write(&wrong, b"another game").expect("write");
        let port = free_port();
        let host_swf = swf.clone();
        let host = thread::spawn(move || play(opts(Role::HostDirect { port }, &host_swf, 0), 300, None));
        thread::sleep(Duration::from_millis(300));
        let addr = format!("127.0.0.1:{port}");
        let refused = Session::start(opts(Role::JoinDirect { addr: addr.clone() }, &wrong, 0));
        assert!(refused.is_err());
        let (join_state, join_session) = play(opts(Role::JoinDirect { addr }, &swf, 0), 300, None);
        let (host_state, _) = host.join().expect("host");
        assert_eq!(join_state.closed, None);
        assert_eq!(host_state.closed, None);
        assert_eq!(join_session.local_index, 1);
    }

    #[test]
    fn different_player_settings_are_refused_by_name() {
        let dir = tempfile::tempdir().expect("temp dir");
        let game = dir.path().join("game.swf");
        std::fs::write(&game, b"one game").expect("write");
        let port = free_port();
        let host_game = game.clone();
        let host = thread::spawn(move || {
            let mut o = opts(Role::HostDirect { port }, &host_game, 0);
            o.host_wait = Some(Duration::from_secs(3));
            Session::start(o).err().map(|e| e.to_string())
        });
        thread::sleep(Duration::from_millis(300));
        let mut friend = opts(Role::JoinDirect { addr: format!("127.0.0.1:{port}") }, &game, 0);
        friend.settings = "frame_rate=Some(60.0); player_version=None".into();
        let err = Session::start(friend).err().expect("different settings must be refused");
        assert!(err.to_string().contains("frame_rate"), "{err}");
        host.join().expect("host");
    }

    #[test]
    fn different_game_file_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (a, b) = (dir.path().join("a.swf"), dir.path().join("b.swf"));
        std::fs::write(&a, b"one game").expect("write");
        std::fs::write(&b, b"another game").expect("write");
        let port = free_port();
        let host = thread::spawn(move || {
            let mut o = opts(Role::HostDirect { port }, &a, 0);
            o.host_wait = Some(Duration::from_secs(3));
            Session::start(o).err().map(|e| e.to_string())
        });
        thread::sleep(Duration::from_millis(300));
        let addr = format!("127.0.0.1:{port}");
        let err = Session::start(opts(Role::JoinDirect { addr }, &b, 0))
            .err()
            .expect("a different game file must be refused");
        assert!(err.to_string().contains("different game file"), "{err}");
        assert!(host.join().expect("host").is_some(), "host should give up after host_wait");
    }
}
