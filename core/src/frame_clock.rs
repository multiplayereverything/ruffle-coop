//! A clock that moves by whole frames instead of following the real clock.
//!
//! Ruffle Together runs the same game on several PCs in lockstep. Anything the game can read
//! from the real clock (getTimer, Date, the Math.random seed) would differ between PCs, so in
//! a session it reads this clock instead, which only moves when a frame runs.

use crate::backend::locale::LocaleBackend;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct FrameClock {
    start_unix_ms: i64,
    elapsed: Duration,
}

impl FrameClock {
    /// `start_unix_ms` is the date the game sees at frame 0 (the host picks it for everyone).
    pub fn new(start_unix_ms: i64) -> Self {
        Self {
            start_unix_ms,
            elapsed: Duration::ZERO,
        }
    }

    pub fn advance(&mut self, frame: Duration) {
        self.elapsed += frame;
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn now_utc(&self) -> DateTime<Utc> {
        let ms = self.start_unix_ms + self.elapsed.as_millis() as i64;
        Utc.timestamp_millis_opt(ms).single().unwrap_or_default()
    }
}

/// Date and the RNG seed (`avm_rng::get_seed` reads the locale) follow the frame clock.
/// The timezone is UTC everywhere, so players in different countries see the same date.
pub struct FrameClockLocale {
    clock: Arc<Mutex<FrameClock>>,
}

impl FrameClockLocale {
    pub fn new(clock: Arc<Mutex<FrameClock>>) -> Self {
        Self { clock }
    }
}

impl LocaleBackend for FrameClockLocale {
    fn get_current_date_time(&self) -> DateTime<Utc> {
        self.clock
            .lock()
            .map(|clock| clock.now_utc())
            .unwrap_or_default()
    }

    fn get_timezone(&self) -> FixedOffset {
        FixedOffset::east_opt(0).expect("UTC offset is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_adds_exact_frames() {
        let mut clock = FrameClock::new(1_700_000_000_000);
        for _ in 0..25 {
            clock.advance(Duration::from_millis(40));
        }
        assert_eq!(clock.elapsed(), Duration::from_millis(1000));
        assert_eq!(clock.now_utc().timestamp_millis(), 1_700_000_001_000);
    }

    #[test]
    fn locale_follows_clock() {
        let clock = Arc::new(Mutex::new(FrameClock::new(1_700_000_000_000)));
        let locale = FrameClockLocale::new(clock.clone());
        clock.lock().unwrap().advance(Duration::from_millis(40));
        assert_eq!(
            locale.get_current_date_time().timestamp_millis(),
            1_700_000_000_040
        );
        assert_eq!(locale.get_timezone().local_minus_utc(), 0);
    }
}
