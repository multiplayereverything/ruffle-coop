//! Messages between Ruffle Together players: one text line each, `|`-separated, so they fit the
//! relay's star rooms (NUL-framed lines) as well as direct connections.

use crate::coop::lockstep::InputFrame;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// First message both ways: a different build or game file is refused before playing.
    Hello {
        build: String,
        swf_sha256: String,
        name: String,
    },
    /// Host to everyone: what both games need to start identically.
    Start {
        player_count: u8,
        your_index: u8,
        delay: u32,
        start_unix_ms: i64,
        frame_rate_milli: u32,
        save: Vec<u8>,
    },
    Input {
        player: u8,
        frame: u32,
        input: InputFrame,
    },
    /// The game's fingerprint at a frame, for the sync check.
    Print { player: u8, frame: u32, hash: u64 },
    Bye { player: u8, reason: String },
    Refuse { reason: String },
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
            name,
        } => format!("HI|{}|{}|{}", clean(build), clean(swf_sha256), clean(name)),
        Msg::Start {
            player_count,
            your_index,
            delay,
            start_unix_ms,
            frame_rate_milli,
            save,
        } => {
            let hex: String = save.iter().map(|b| format!("{b:02x}")).collect();
            format!("ST|{player_count}|{your_index}|{delay}|{start_unix_ms}|{frame_rate_milli}|{hex}")
        }
        Msg::Input {
            player,
            frame,
            input,
        } => match input.mouse {
            Some((x, y, down)) => {
                format!("IN|{player}|{frame}|{}|{x}|{y}|{}", input.buttons, down as u8)
            }
            None => format!("IN|{player}|{frame}|{}|-|-|-", input.buttons),
        },
        Msg::Print {
            player,
            frame,
            hash,
        } => format!("FP|{player}|{frame}|{hash:016x}"),
        Msg::Bye { player, reason } => format!("BY|{player}|{}", clean(reason)),
        Msg::Refuse { reason } => format!("NO|{}", clean(reason)),
    }
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
            want(4)?;
            Ok(Msg::Hello {
                build: text(1)?,
                swf_sha256: text(2)?,
                name: text(3)?,
            })
        }
        "ST" => {
            want(7)?;
            let hex = parts[6];
            if hex.len() % 2 != 0 {
                return Err(bad());
            }
            let save = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|_| bad())?;
            Ok(Msg::Start {
                player_count: byte(1)?,
                your_index: byte(2)?,
                delay: uint(3)?,
                start_unix_ms: num(4)?,
                frame_rate_milli: uint(5)?,
                save,
            })
        }
        "IN" => {
            want(7)?;
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
                input: InputFrame { buttons, mouse },
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
            want(2)?;
            Ok(Msg::Refuse { reason: text(1)? })
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
            name: "Liam".into(),
        });
        roundtrip(Msg::Start {
            player_count: 2,
            your_index: 1,
            delay: 3,
            start_unix_ms: 1_700_000_000_000,
            frame_rate_milli: 25_000,
            save: vec![0, 1, 254, 255],
        });
        roundtrip(Msg::Start {
            player_count: 4,
            your_index: 3,
            delay: 0,
            start_unix_ms: -5,
            frame_rate_milli: 30_000,
            save: vec![],
        });
        roundtrip(Msg::Input {
            player: 1,
            frame: 42,
            input: InputFrame {
                buttons: UP | FIRE,
                mouse: None,
            },
        });
        roundtrip(Msg::Input {
            player: 0,
            frame: 7,
            input: InputFrame {
                buttons: 0,
                mouse: Some((-20, 11000, true)),
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
            reason: "different game file".into(),
        });
    }

    #[test]
    fn bad_lines_are_errors() {
        assert!(decode("IN|0|x|0|-|-|-").is_err());
        assert!(decode("IN|0|1").is_err());
        assert!(decode("ZZ|1").is_err());
        assert!(decode("").is_err());
        assert!(decode("ST|2|1|3|0|25000|zz").is_err());
    }

    #[test]
    fn names_are_cleaned() {
        let line = encode(&Msg::Hello {
            build: "b".into(),
            swf_sha256: "s".into(),
            name: "a|b\nc\0d".into(),
        });
        assert_eq!(
            decode(&line),
            Ok(Msg::Hello {
                build: "b".into(),
                swf_sha256: "s".into(),
                name: "a b c d".into(),
            })
        );
    }
}
