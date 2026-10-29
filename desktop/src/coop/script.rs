//! Scripted input for tests and labs, so nothing ever touches the real keyboard or mouse.
//!
//! ```text
//! # comment
//! p0 UP|FIRE      player 0 holds these from now on (UP DOWN LEFT RIGHT FIRE ALT KEY1-KEY4 VIEW
//!                 QUIT YES NO MENU, or 0)
//! wait 25         25 frames with what is held now
//! click 275 300   host mouse: one frame pressed at stage pixel (275, 300), then released
//! move 100 50     host mouse moves (no click)
//! move 100 50 2   player 2's mouse (any of click/move; the host's when left out)
//! type 1 Sam{Enter}  player 1 types "Sam" then Enter (8 characters a frame; {Backspace} too)
//! ```

use crate::coop::lockstep::{InputFrame, MAX_PLAYERS, Typed, button_named, text_control_char};
use ruffle_core::events::TextControlCode;

pub struct Script {
    frames: Vec<[InputFrame; MAX_PLAYERS]>,
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut frames = vec![];
        let mut now = [InputFrame::default(); MAX_PLAYERS];
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let fail = |why: &str| format!("line {}: {why}: {line}", number + 1);
            let words: Vec<&str> = line.split_whitespace().collect();
            let int = |i: usize| -> Result<i32, String> {
                words.get(i).and_then(|w| w.parse().ok()).ok_or_else(|| fail("expected a number"))
            };
            match words[0] {
                "wait" => {
                    let n = int(1)?;
                    frames.extend(std::iter::repeat_n(now, n.max(0) as usize));
                }
                "click" | "move" => {
                    let (x, y) = (int(1)? * 20, int(2)?.saturating_mul(20));
                    // a slot's mouse (default: the host's)
                    let slot = match words.get(3) {
                        Some(_) => int(3)? as usize,
                        None => 0,
                    };
                    if slot >= MAX_PLAYERS {
                        return Err(fail("unknown player"));
                    }
                    if words[0] == "click" {
                        now[slot].mouse = Some((x, y, true));
                        frames.push(now);
                    }
                    now[slot].mouse = Some((x, y, false));
                }
                "type" => {
                    let slot = int(1)? as usize;
                    if slot >= MAX_PLAYERS {
                        return Err(fail("unknown player"));
                    }
                    // the rest of the line after "type N ", spaces included
                    let text = line.splitn(3, ' ').nth(2).ok_or_else(|| fail("expected text"))?;
                    let mut chars = vec![];
                    let mut rest = text;
                    while let Some(c) = rest.chars().next() {
                        let named = [("{Enter}", TextControlCode::Enter), ("{Backspace}", TextControlCode::Backspace)]
                            .into_iter()
                            .find(|(name, _)| rest.starts_with(name));
                        if let Some((name, code)) = named {
                            chars.push(text_control_char(code));
                            rest = &rest[name.len()..];
                        } else {
                            chars.push(c);
                            rest = &rest[c.len_utf8()..];
                        }
                    }
                    for chunk in chars.chunks(Typed::MAX) {
                        let mut frame = now;
                        frame[slot].text = Typed::from_chars(chunk.iter().copied());
                        frames.push(frame);
                    }
                }
                word if word.starts_with('p') => {
                    let player: usize = word[1..]
                        .parse()
                        .ok()
                        .filter(|&p| p < MAX_PLAYERS)
                        .ok_or_else(|| fail("unknown player"))?;
                    let mut buttons = 0;
                    for name in words.get(1).ok_or_else(|| fail("expected buttons"))?.split('|') {
                        buttons |= button_named(name).ok_or_else(|| fail("unknown button"))?;
                    }
                    now[player].buttons = buttons;
                }
                _ => return Err(fail("unknown command")),
            }
        }
        Ok(Script { frames })
    }

    /// Number of scripted frames.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// `player`'s input at `frame`; empty after the script ends.
    pub fn input(&self, frame: usize, player: usize) -> InputFrame {
        self.frames
            .get(frame)
            .map(|row| row[player])
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coop::lockstep::{FIRE, KEY1, LEFT, NO, UP, VIEW};

    #[test]
    fn type_types_text_once_eight_characters_a_frame() {
        let script = Script::parse("p0 UP\ntype 1 Sam Smith!{Enter}\nwait 1").expect("parses");
        assert_eq!(script.len(), 3, "10 characters + Enter: two frames, then the wait");
        let typed: String = (0..2).flat_map(|f| script.input(f, 1).text.chars().to_vec()).collect();
        assert_eq!(typed, format!("Sam Smith!{}", text_control_char(TextControlCode::Enter)));
        assert_eq!(script.input(0, 0).buttons, UP, "held buttons go on");
        assert!(script.input(2, 1).text.chars().is_empty(), "typed once, not held");
        assert!(Script::parse("type 9 x").is_err());
        assert!(Script::parse("type 1").is_err());
    }

    #[test]
    fn wait_repeats_current_buttons() {
        let script = Script::parse("p0 UP|FIRE\np1 LEFT\nwait 3\np0 0\nwait 1").expect("parses");
        assert_eq!(script.len(), 4);
        assert_eq!(script.input(0, 0).buttons, UP | FIRE);
        assert_eq!(script.input(2, 1).buttons, LEFT);
        assert_eq!(script.input(3, 0).buttons, 0);
        assert_eq!(script.input(3, 1).buttons, LEFT);
    }

    #[test]
    fn click_is_one_frame_down_then_up() {
        let script = Script::parse("click 275 300\nwait 2").expect("parses");
        assert_eq!(script.len(), 3);
        assert_eq!(script.input(0, 0).mouse, Some((275 * 20, 300 * 20, true)));
        assert_eq!(script.input(1, 0).mouse, Some((275 * 20, 300 * 20, false)));
        // only the host has a mouse
        assert_eq!(script.input(0, 1).mouse, None);
    }

    #[test]
    fn past_the_end_is_empty_input() {
        let script = Script::parse("p0 UP\nwait 1").expect("parses");
        assert_eq!(script.input(99, 0), InputFrame::default());
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let script = Script::parse("# start\n\nwait 2 # two frames").expect("parses");
        assert_eq!(script.len(), 2);
    }

    #[test]
    fn mistakes_name_the_line() {
        // any slot's mouse: `move X Y slot`, `click X Y slot` (no slot: the host's)
        let script = Script::parse("move 10 20 2\nwait 1\nclick 5 6 1\nwait 1").expect("parses");
        assert_eq!(script.input(0, 2).mouse, Some((200, 400, false)));
        assert_eq!(script.input(1, 1).mouse, Some((100, 120, true)));
        assert_eq!(script.input(2, 1).mouse, Some((100, 120, false)));
        assert_eq!(script.input(2, 0).mouse, None);
        let script = Script::parse("p1 KEY1|VIEW|NO\nwait 1").expect("parses");
        assert_eq!(script.input(0, 1).buttons, KEY1 | VIEW | NO);
        let err = Script::parse("wait 1\np0 JUMP").err().expect("an error");
        assert!(err.contains("line 2"), "{err}");
        assert!(Script::parse("p9 UP").is_err());
        assert!(Script::parse("click 1").is_err());
    }
}
