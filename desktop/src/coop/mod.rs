//! Ruffle Together: lockstep online play for local-multiplayer Flash games.
//! Every PC runs the same game; only inputs travel, and a frame runs once all players' inputs for it are in.
// Until the session is wired into the app (plan Task 10), parts are only used by tests.
#![allow(dead_code)]

pub mod frame;
pub mod keymap;
pub mod lockstep;
pub mod net;
pub mod protocol;
pub mod script;
pub mod storage;
pub mod replay;

use std::path::PathBuf;

#[derive(clap::Args, Debug, Clone, Default)]
pub struct CoopOpt {
    /// Testing: run SWF headless with scripted input (--coop-script) and write the game's
    /// fingerprint for every frame (--coop-out), to prove the game runs the same every time.
    #[clap(long, value_name = "SWF", help_heading = "Ruffle Together")]
    pub coop_replay: Option<PathBuf>,

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
