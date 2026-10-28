//! Ruffle Together: lockstep online play for local-multiplayer Flash games.
//! Every PC runs the same game; only inputs travel, and a frame runs once all players' inputs for it are in.

pub mod frame;
pub mod keymap;
pub mod lockstep;
pub mod net;
pub mod protocol;
pub mod script;
pub mod session;
pub mod storage;
pub mod replay;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// The session this process plays in, if any (set once, before the window opens).
static SESSION: OnceLock<Mutex<session::Session>> = OnceLock::new();

pub fn set_session(session: session::Session) {
    let _ = SESSION.set(Mutex::new(session));
}

/// The running session; None in normal (single-player) use, where nothing changes.
pub fn session() -> Option<MutexGuard<'static, session::Session>> {
    SESSION.get().and_then(|session| session.lock().ok())
}

#[derive(clap::Args, Debug, Clone, Default)]
pub struct CoopOpt {
    /// Testing: run SWF headless with scripted input (--coop-script) and write the game's
    /// fingerprint for every frame (--coop-out), to prove the game runs the same every time.
    #[clap(long, value_name = "SWF", help_heading = "Ruffle Together")]
    pub coop_replay: Option<PathBuf>,

    /// Host a session; friends connect directly to this port (LAN, Radmin, or a forwarded port).
    #[clap(long, value_name = "PORT", help_heading = "Ruffle Together")]
    pub coop_host: Option<u16>,

    /// Host a session through a relay (host:port); prints the room code friends type.
    #[clap(long, value_name = "RELAY", help_heading = "Ruffle Together")]
    pub coop_host_relay: Option<String>,

    /// Join a session directly (host IP:port).
    #[clap(long, value_name = "ADDRESS", help_heading = "Ruffle Together")]
    pub coop_join: Option<String>,

    /// Join a session through a relay (host:port), with --coop-code.
    #[clap(long, value_name = "RELAY", help_heading = "Ruffle Together")]
    pub coop_join_relay: Option<String>,

    /// The room code to join through the relay.
    #[clap(long, value_name = "CODE", help_heading = "Ruffle Together")]
    pub coop_code: Option<String>,

    /// Host: input delay in frames (the same for every player).
    #[clap(long, default_value_t = 3, help_heading = "Ruffle Together")]
    pub coop_delay: u32,

    /// Your name, shown to the other players.
    #[clap(long, default_value = "Player", help_heading = "Ruffle Together")]
    pub coop_name: String,

    /// Testing: play the session without a window for --coop-frames frames, then print the
    /// result (sync, input delay) and exit.
    #[clap(long, help_heading = "Ruffle Together")]
    pub coop_headless: bool,

    /// Testing: scripted input instead of the keyboard and mouse.
    #[clap(long, value_name = "FILE", help_heading = "Ruffle Together")]
    pub coop_script: Option<PathBuf>,

    /// Replay: number of frames to run.
    #[clap(long, default_value_t = 1500, help_heading = "Ruffle Together")]
    pub coop_frames: u32,

    /// Replay: sleep a random 0..N ms between frames (the result must not change).
    #[clap(long, default_value_t = 0, help_heading = "Ruffle Together")]
    pub coop_jitter_ms: u64,

    /// Replay: use the real clock (upstream behavior), as the control run that must differ.
    #[clap(long, help_heading = "Ruffle Together")]
    pub coop_no_clock: bool,

    /// Replay: the date the game sees at frame 0, in Unix ms (sessions use the host's).
    #[clap(long, default_value_t = 1_700_000_000_000, help_heading = "Ruffle Together")]
    pub coop_start_ms: i64,

    /// Replay: where to write "frame fingerprint" lines.
    #[clap(long, value_name = "FILE", help_heading = "Ruffle Together")]
    pub coop_out: Option<PathBuf>,

    /// Replay: frames to save as PNG next to --coop-out, e.g. 100,250.
    #[clap(long, value_delimiter = ',', help_heading = "Ruffle Together")]
    pub coop_shot: Vec<u32>,
}

impl CoopOpt {
    /// The session these options ask for, if any.
    pub fn role(&self) -> anyhow::Result<Option<net::Role>> {
        use net::Role;
        let roles = [
            self.coop_host.map(|port| Role::HostDirect { port }),
            self.coop_host_relay.clone().map(|relay| Role::HostRelay { relay }),
            self.coop_join.clone().map(|addr| Role::JoinDirect { addr }),
            self.coop_join_relay.clone().map(|relay| Role::JoinRelay {
                relay,
                code: self.coop_code.clone().unwrap_or_default().to_uppercase(),
            }),
        ];
        let mut roles = roles.into_iter().flatten();
        let role = roles.next();
        if roles.next().is_some() {
            anyhow::bail!("choose one of --coop-host, --coop-host-relay, --coop-join, --coop-join-relay");
        }
        if matches!(role, Some(Role::JoinRelay { ref code, .. }) if code.is_empty()) {
            anyhow::bail!("--coop-join-relay needs --coop-code");
        }
        Ok(role)
    }
}
