//! Messages between Ruffle Together players: one text line each, `|`-separated, so they fit the
//! relay's star rooms (NUL-framed lines) as well as direct connections.

use crate::coop::lockstep::{InputFrame, Typed};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// First message both ways: a different build or game file is refused before playing.
    Hello {
        build: String,
        swf_sha256: String,
        /// The player settings that change how a game behaves (frame rate, Flash version, ...).
        settings: String,
        name: String,
        /// A random number naming this player until the start (0: the host); through the relay
        /// a friend doesn't know its connection number.
        id: u64,
    },
    /// Host to everyone: what every game needs to start identically.
    Start {
        delay: u32,
        start_unix_ms: i64,
        frame_rate_milli: u32,
        save: Vec<u8>,
        /// Every player in slot order (the host first): their Hello id and name.
        players: Vec<(u64, String)>,
        /// The host's game settings (`coop.option`).
        options: Vec<(String, String)>,
    },
    Input {
        player: u8,
        frame: u32,
        input: InputFrame,
    },
    /// The game's fingerprint at a frame, for the sync check.
    Print { player: u8, frame: u32, hash: u64 },
    Bye { player: u8, reason: String },
    /// Host to the joiner with this Hello id (the others hear it too and ignore it).
    Refuse { id: u64, reason: String },
    /// Host to a friend before the start: the round trip picks the input delay.
    Ping { seq: u32 },
    Pong { seq: u32 },
    /// Sent every second by a thread of its own, whatever the game does: the player's Ruffle is
    /// running, and its game is at `frame` (a frozen game keeps sending the same frame).
    Alive { player: u8, frame: u32 },
}

/// Names and reasons can't carry the separator or line breaks.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c == '|' || c.is_control() { ' ' } else { c })
        .collect()
}

pub fn encode(msg: &Msg) -> String {
    match msg {
        Msg::Hello {
            build,
            swf_sha256,
            settings,
            name,
            id,
        } => format!(
            "HI|{}|{}|{}|{}|{id:x}",
            clean(build),
            clean(swf_sha256),
            clean(settings),
            clean(name)
        ),
        Msg::Start {
            delay,
            start_unix_ms,
            frame_rate_milli,
            save,
            players,
            options,
        } => {
            let hex: String = save.iter().map(|b| format!("{b:02x}")).collect();
            let options: String = options
                .iter()
                .map(|(name, value)| format!("{name}={value}\n"))
                .collect::<String>()
                .bytes()
                .map(|b| format!("{b:02x}"))
                .collect();
            let players: Vec<String> = players
                .iter()
                .map(|(id, name)| format!("{id:x}:{}", clean(name).replace(',', " ")))
                .collect();
            format!("ST|{delay}|{start_unix_ms}|{frame_rate_milli}|{hex}|{}|{options}", players.join(","))
        }
        Msg::Input {
            player,
            frame,
            input,
        } => {
            let text: String = input.text.chars().iter().collect();
            let text: String = text.bytes().map(|b| format!("{b:02x}")).collect();
            match input.mouse {
                Some((x, y, down)) => {
                    format!("IN|{player}|{frame}|{}|{x}|{y}|{}|{text}", input.buttons, down as u8)
                }
                None => format!("IN|{player}|{frame}|{}|-|-|-|{text}", input.buttons),
            }
        }
        Msg::Print {
            player,
            frame,
            hash,
        } => format!("FP|{player}|{frame}|{hash:016x}"),
        Msg::Bye { player, reason } => format!("BY|{player}|{}", clean(reason)),
        Msg::Refuse { id, reason } => format!("NO|{id:x}|{}", clean(reason)),
        Msg::Ping { seq } => format!("PI|{seq}"),
        Msg::Pong { seq } => format!("PO|{seq}"),
        Msg::Alive { player, frame } => format!("AL|{player}|{frame}"),
    }
}

fn unhex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok()).collect()
}

pub fn decode(line: &str) -> Result<Msg, String> {
    let parts: Vec<&str> = line.split('|').collect();
    let bad = || format!("bad message: {line:.80}");
    let num = |i: usize| -> Result<i64, String> {
        parts.get(i).and_then(|p| p.parse::<i64>().ok()).ok_or_else(bad)
    };
    let text = |i: usize| -> Result<String, String> {
        parts.get(i).map(|p| p.to_string()).ok_or_else(bad)
    };
    let byte = |i: usize| -> Result<u8, String> { u8::try_from(num(i)?).map_err(|_| bad()) };
    let uint = |i: usize| -> Result<u32, String> { u32::try_from(num(i)?).map_err(|_| bad()) };
    let want = |n: usize| -> Result<(), String> {
        if parts.len() == n { Ok(()) } else { Err(bad()) }
    };
    match parts[0] {
        "HI" => {
            want(6)?;
            Ok(Msg::Hello {
                build: text(1)?,
                swf_sha256: text(2)?,
                settings: text(3)?,
                name: text(4)?,
                id: u64::from_str_radix(parts[5], 16).map_err(|_| bad())?,
            })
        }
        "ST" => {
            want(7)?;
            let hex = parts[4];
            if !hex.len().is_multiple_of(2) {
                return Err(bad());
            }
            let save = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|_| bad())?;
            let players = parts[5]
                .split(',')
                .map(|player| {
                    let (id, name) = player.split_once(':')?;
                    Some((u64::from_str_radix(id, 16).ok()?, name.to_string()))
                })
                .collect::<Option<Vec<_>>>()
                .filter(|players| (1..=crate::coop::lockstep::MAX_PLAYERS).contains(&players.len()))
                .ok_or_else(bad)?;
            let options = unhex(parts[6]).ok_or_else(bad)?;
            let options = String::from_utf8(options).map_err(|_| bad())?;
            let options = options
                .lines()
                .map(|line| line.split_once('=').map(|(n, v)| (n.to_string(), v.to_string())))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(bad)?;
            Ok(Msg::Start {
                delay: uint(1)?,
                start_unix_ms: num(2)?,
                frame_rate_milli: uint(3)?,
                save,
                players,
                options,
            })
        }
        "IN" => {
            want(8)?;
            let hex = parts[7];
            let bytes = (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(hex.get(i * 2..i * 2 + 2).unwrap_or("zz"), 16))
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|_| bad())?;
            let text = String::from_utf8(bytes).map_err(|_| bad())?;
            if hex.len() % 2 != 0 || text.chars().count() > Typed::MAX {
                return Err(bad());
            }
            let buttons = u16::try_from(num(3)?).map_err(|_| bad())?;
            let mouse = if parts[4] == "-" {
                None
            } else {
                let x = i32::try_from(num(4)?).map_err(|_| bad())?;
                let y = i32::try_from(num(5)?).map_err(|_| bad())?;
                Some((x, y, num(6)? != 0))
            };
            Ok(Msg::Input {
                player: byte(1)?,
                frame: uint(2)?,
                input: InputFrame { buttons, mouse, text: Typed::from_chars(text.chars()) },
            })
        }
        "FP" => {
            want(4)?;
            Ok(Msg::Print {
                player: byte(1)?,
                frame: uint(2)?,
                hash: u64::from_str_radix(parts[3], 16).map_err(|_| bad())?,
            })
        }
        "BY" => {
            want(3)?;
            Ok(Msg::Bye {
                player: byte(1)?,
                reason: text(2)?,
            })
        }
        "NO" => {
            want(3)?;
            Ok(Msg::Refuse {
                id: u64::from_str_radix(parts[1], 16).map_err(|_| bad())?,
                reason: text(2)?,
            })
        }
        "PI" => {
            want(2)?;
            Ok(Msg::Ping { seq: uint(1)? })
        }
        "PO" => {
            want(2)?;
            Ok(Msg::Pong { seq: uint(1)? })
        }
        "AL" => {
            want(3)?;
            Ok(Msg::Alive {
                player: byte(1)?,
                frame: uint(2)?,
            })
        }
        _ => Err(bad()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coop::lockstep::{FIRE, UP};

    fn roundtrip(msg: Msg) {
        let line = encode(&msg);
        assert!(!line.contains('\0') && !line.contains('\n'), "{line}");
        assert_eq!(decode(&line), Ok(msg));
    }

    #[test]
    fn every_message_roundtrips() {
        roundtrip(Msg::Hello {
            build: "0.1.0#abc123".into(),
            swf_sha256: "4635ddfc".into(),
            settings: "frame_rate=None; quality=None".into(),
            name: "Liam".into(),
            id: u64::MAX,
        });
        roundtrip(Msg::Start {
            delay: 3,
            start_unix_ms: 1_700_000_000_000,
            frame_rate_milli: 25_000,
            save: vec![0, 1, 254, 255],
            players: vec![(0, "Liam".into()), (77, "Sam".into())],
            options: vec![("map".into(), "3".into()), ("note".into(), "a|b=c,d".into())],
        });
        roundtrip(Msg::Start {
            delay: 0,
            start_unix_ms: -5,
            frame_rate_milli: 30_000,
            save: vec![],
            players: vec![(0, String::new()), (1, "a:b".into()), (2, "c".into()), (3, "d".into())],
            options: vec![],
        });
        roundtrip(Msg::Input {
            player: 1,
            frame: 42,
            input: InputFrame {
                buttons: UP | FIRE,
                mouse: None,
                text: Typed::from_chars("Li|am\u{E014}é".chars()),
            },
        });
        roundtrip(Msg::Input {
            player: 0,
            frame: 7,
            input: InputFrame {
                buttons: 0,
                mouse: Some((-20, 11000, true)),
                ..Default::default()
            },
        });
        roundtrip(Msg::Print {
            player: 1,
            frame: 25,
            hash: u64::MAX,
        });
        roundtrip(Msg::Bye {
            player: 1,
            reason: "closed the game".into(),
        });
        roundtrip(Msg::Refuse {
            id: 0xabc,
            reason: "different game file".into(),
        });
        roundtrip(Msg::Ping { seq: 4 });
        roundtrip(Msg::Pong { seq: 4 });
        roundtrip(Msg::Alive { player: 2, frame: 3024 });
    }

    #[test]
    fn bad_lines_are_errors() {
        assert!(decode("IN|0|x|0|-|-|-").is_err());
        assert!(decode("IN|0|1").is_err());
        assert!(decode("ZZ|1").is_err());
        assert!(decode("").is_err());
        assert!(decode("ST|3|0|25000|zz|0:a").is_err());
        assert!(decode("ST|3|0|25000||").is_err(), "no players");
        assert!(decode("ST|3|0|25000||x:a").is_err());
    }

    #[test]
    fn names_are_cleaned() {
        let line = encode(&Msg::Hello {
            build: "b".into(),
            swf_sha256: "s".into(),
            settings: String::new(),
            name: "a|b\nc\0d".into(),
            id: 1,
        });
        assert_eq!(
            decode(&line),
            Ok(Msg::Hello {
                build: "b".into(),
                swf_sha256: "s".into(),
                settings: String::new(),
                name: "a b c d".into(),
                id: 1,
            })
        );
        // the player list in Start is split on commas
        let line = encode(&Msg::Start {
            delay: 1,
            start_unix_ms: 0,
            frame_rate_milli: 0,
            save: vec![],
            players: vec![(0, "x,y".into())],
            options: vec![],
        });
        assert!(matches!(decode(&line), Ok(Msg::Start { players, .. }) if players == vec![(0, "x y".into())]));
    }
}
