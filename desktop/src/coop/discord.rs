//! Discord Rich Presence during a session ("Playing Gun Mayhem 2 - room ABCDE, 2 players"), on
//! this PC's Discord only. It never touches the game; without Discord it does nothing.

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::sync::OnceLock;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

/// Ruffle Together's Discord application id (made at discord.com/developers). While it's this
/// placeholder, presence is off.
pub const APP_ID: &str = "REPLACE_WITH_DISCORD_APPLICATION_ID";

static PRESENCE: OnceLock<Option<Sender<(String, String)>>> = OnceLock::new();

/// Shows `details` and `state` on this PC's Discord profile (the newest call wins).
pub fn show(details: &str, state: &str) {
    let sender = PRESENCE.get_or_init(|| {
        APP_ID.parse::<u64>().ok()?;
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("discord-presence".into())
            .spawn(move || run(receiver))
            .ok()?;
        Some(sender)
    });
    if let Some(sender) = sender {
        let _ = sender.send((details.to_string(), state.to_string()));
    }
}

/// Talks to Discord on its own thread: Discord can be slow, closed, or started later, and the
/// game must never wait for it. Every 15 s it tries again (and re-sends) if it isn't connected.
fn run(receiver: mpsc::Receiver<(String, String)>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut latest: Option<(String, String)> = None;
    loop {
        match receiver.recv_timeout(Duration::from_secs(15)) {
            Ok(presence) => latest = Some(presence),
            Err(RecvTimeoutError::Timeout) if client.is_some() => continue,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let Some((details, state)) = &latest else {
            continue;
        };
        if client.is_none() {
            let mut new_client = DiscordIpcClient::new(APP_ID);
            if new_client.connect().is_err() {
                continue;
            }
            client = Some(new_client);
        }
        let activity = activity::Activity::new().details(details.as_str()).state(state.as_str());
        if let Some(connected) = client.as_mut()
            && connected.set_activity(activity).is_err()
        {
            // Discord was closed: connect again next time
            client = None;
        }
    }
    if let Some(mut connected) = client {
        let _ = connected.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the placeholder id nothing starts: no thread, no connection attempts.
    #[test]
    fn the_placeholder_id_turns_presence_off() {
        assert!(APP_ID.parse::<u64>().is_err());
        show("Playing a test", "room ABCDE");
        assert!(matches!(PRESENCE.get(), Some(None)));
    }
}
