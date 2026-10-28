//! Ruffle Together: lockstep online play for local-multiplayer Flash games.
//! Every PC runs the same game; only inputs travel, and a frame runs once all players' inputs for it are in.
// Until the session is wired into the app (plan Task 10), parts are only used by tests.
#![allow(dead_code)]

pub mod keymap;
pub mod lockstep;
pub mod net;
pub mod protocol;
pub mod storage;
