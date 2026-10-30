//! Discord Rich Presence during a session ("Playing Gun Mayhem 2 - room ABCDE, 2 of 4"), with a
//! Join button for relay rooms, on this PC's Discord only. It never touches the game; without
//! Discord it does nothing.

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::sync::OnceLock;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

/// Ruffle Together's Discord application id (discord.com/developers). Presence is off while
/// it isn't a number.
pub const APP_ID: &str = "1565524080453755040";

/// The web page a Join button opens, followed by `<mod id>/<room code>`. Discord buttons only
/// open https links, so this page opens `ruffle-together://join/<mod id>/<room code>` for the
/// launcher. While it's this placeholder there is no Join button.
pub const JOIN_PAGE: &str = "https://REPLACE_WITH_JOIN_PAGE/join/";

/// What Discord shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Presence {
    pub details: String,
    pub state: String,
    /// Players in the game, and how many it takes.
    pub party: Option<(u8, u8)>,
    /// The Join button's link, for a relay room the launcher can join.
    pub join: Option<String>,
}

/// The Join button's link for a room, if the join page is set up.
pub fn join_link(mod_id: &str, code: &str) -> Option<String> {
    if JOIN_PAGE.contains("REPLACE_WITH") {
        return None;
    }
    Some(join_link_on(JOIN_PAGE, mod_id, code))
}

fn join_link_on(page: &str, mod_id: &str, code: &str) -> String {
    let safe = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect::<String>();
    format!("{page}{}/{}", safe(mod_id), safe(code))
}

static PRESENCE: OnceLock<Option<Sender<Presence>>> = OnceLock::new();

/// Shows `presence` on this PC's Discord profile (the newest call wins).
pub fn show(presence: Presence) {
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
        let _ = sender.send(presence);
    }
}

/// Talks to Discord on its own thread: Discord can be slow, closed, or started later, and the
/// game must never wait for it. Every 15 s it tries again (and re-sends) if it isn't connected.
fn run(receiver: mpsc::Receiver<Presence>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut latest: Option<Presence> = None;
    loop {
        match receiver.recv_timeout(Duration::from_secs(15)) {
            Ok(presence) => latest = Some(presence),
            Err(RecvTimeoutError::Timeout) if client.is_some() => continue,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let Some(presence) = &latest else {
            continue;
        };
        if client.is_none() {
            let mut new_client = DiscordIpcClient::new(APP_ID);
            if new_client.connect().is_err() {
                continue;
            }
            tracing::info!("Ruffle Together: Discord presence on");
            client = Some(new_client);
        }
        let mut activity = activity::Activity::new().details(presence.details.as_str()).state(presence.state.as_str());
        // the app's art asset named "logo" (Discord developer portal, Rich Presence, Art Assets)
        activity = activity.assets(activity::Assets::new().large_image("logo").large_text("Ruffle Together"));
        if let Some((players, max)) = presence.party {
            activity = activity.party(activity::Party::new().size([players.into(), max.into()]));
        }
        if let Some(join) = &presence.join {
            activity = activity.buttons(vec![activity::Button::new("Join game", join.as_str())]);
        }
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

    /// The app id turns presence on; with the placeholder page there is no Join button.
    #[test]
    fn app_id_set_and_join_off_until_the_page_exists() {
        assert!(APP_ID.parse::<u64>().is_ok());
        assert_eq!(join_link("gm2", "ABCDE"), None);
    }

    #[test]
    fn join_links_carry_only_safe_characters() {
        assert_eq!(join_link_on("https://x.example/join/", "gm2", "ABCDE"), "https://x.example/join/gm2/ABCDE");
        assert_eq!(join_link_on("https://x.example/join/", "../evil?", "AB CD"), "https://x.example/join/evil/ABCD");
    }
}
