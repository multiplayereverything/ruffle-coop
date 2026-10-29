# Ruffle Together

**Play local-multiplayer Flash games online with up to 3 friends.**

Ruffle Together is a fork of [Ruffle](https://github.com/ruffle-rs/ruffle), the Flash Player emulator written in
Rust. Every player runs the same game on their own PC, and only the key presses and mouse travel over the network.
Each frame runs on every PC with every player's input, so the games stay identical with nothing else to sync.

[**Download the latest release**](https://github.com/multiplayereverything/ruffle-coop/releases/latest) (Windows
x64). The easiest way to play is through the Multiplayer Mods Launcher, which keeps Ruffle Together up to date and
starts each game with the right settings.

## Games

Games with a built-in key map (`--coop-keys <name>`):

| Game | Players | Key map |
|---|---|---|
| Bad Ice-Cream | 2 | `bad-ice-cream` (the default) |
| Gun Mayhem 2 | 2-4 | `gun-mayhem-2` |
| Territory War | 2-4 (hot-seat) | `territory-war` |
| Bow Man | 2 | `bow-man` |
| Smileys War | 2-4 | `smileys-war` |
| Stealth Assassin | 2-4 | `stealth-assassin` |
| Stealth Hunter 2 | 2-4 | `stealth-hunter-2` |
| Intrusion / Intrusion 2 | 2-4 | `intrusion`, `intrusion-2` |
| Air Battle | 2-4 | `air-battle` |
| Ultimate Crab Battle | 2-4 | `ultimate-crab-battle` |

Any other game can bring its own key map as a small text file (see [Key maps](#key-maps)).

## Playing

Through the central relay with a room code (no IP or port forwarding needed):

```
ruffle_desktop --coop-host-relay 195.60.166.57:25569 --coop-name Liam Game.swf
ruffle_desktop --coop-join-relay 195.60.166.57:25569 --coop-code ABCDE --coop-name Sam Game.swf
```

Or directly by IP (LAN, a VPN, or a forwarded port):

```
ruffle_desktop --coop-host 7801 --coop-name Liam Game.swf
ruffle_desktop --coop-join <host IP>:7801 --coop-name Sam Game.swf
```

- Everyone needs the **same game file** and the **same Ruffle Together version**. Anything else is refused when
  they connect, with the reason.
- `--coop-players N` (host): wait for N players (2-4). `--coop-keys <name or file>`: everyone uses the same map.
- The host picks a small input delay from the ping automatically (1 frame on a LAN). `--coop-delay N` fixes it.
- Everyone plays with the host's save (unlocked levels). Nobody's own save changes.
- While players join, the window shows the room code and who is in. During the game the title shows your player
  number, and "waiting for...", "... left" or "OUT OF SYNC" when something goes wrong.

## How it works

- **Lockstep:** a frame runs only once every player's input for it has arrived. Inputs are scheduled a few frames
  ahead to hide the network lag.
- **Determinism:** the game sees no real time and nothing else that differs between PCs. `getTimer`, `Date`,
  timers, `Math.random`, sound endings and script limits all follow a frame clock that is the same everywhere. The
  window size doesn't reach the game, and big movies load fully before the first frame.
- **Sync check:** every 25 frames each PC compares a fingerprint of the game: every object's position,
  visibility, depth and frame, plus the clock and random state. If they ever differ, every window says so, and
  each PC writes a report of its game state to find the cause.
- **Connections:** direct TCP, or the central relay with 5-letter room codes. A heartbeat tells the players apart
  when someone's game freezes, crashes or closes. The last few logs are kept in `%LOCALAPPDATA%\ruffle\log\`.

## For game builders

### Key maps

```
# comment
shared                                    every slot presses the same keys (a hot-seat game)
slot 0: UP=38 DOWN=40 LEFT=37 RIGHT=39 FIRE=90 ALT=88
slot 1: UP=87 DOWN=83 LEFT=65 RIGHT=68 FIRE=84 ALT=89
local: UP=38,87 DOWN=40,83 LEFT=37,65 RIGHT=39,68 FIRE=90,84 ALT=88,89
```

- `slot N:` (0-3): what that player's buttons press in the game, as Flash key codes. The number of slots is how
  many players the game can have.
- `local:` the keys a player presses on their own PC for each button.
- Buttons: `UP DOWN LEFT RIGHT FIRE ALT KEY1 KEY2 KEY3 KEY4 VIEW QUIT YES NO`.

The full key-code list is in [the user guide](https://github.com/multiplayereverything/ruffle-coop/blob/coop/docs/ruffle-together.md).

### The co-op API (AS2 and AS3)

A game made for Ruffle Together asks about the players through `ExternalInterface`:

| Call | Returns |
|---|---|
| `coop.players` | number of players (2-4) |
| `coop.localSlot` | this PC's slot (0 = host). Only for what *this* PC shows, never for gameplay |
| `coop.mouseX(slot)`, `coop.mouseY(slot)`, `coop.mouseDown(slot)` | each player's mouse, the same on every PC |

Check `ExternalInterface.available && ExternalInterface.call("coop.players") > 0` to know you're in a session. In
plain Flash or Ruffle, fall back to normal single-player behaviour. AS1/AS2 SWFs older than version 8 must unhide
`ExternalInterface` first; the user guide shows how.

**Coming next:**
- a camera for each player, HUD clips that stay on screen, and name tags above players
- key maps a game defines itself
- a drop-in `RuffleTogether.as` library
- Discord Rich Presence
- playing in the browser

### Testing without anyone playing

- `--coop-replay <swf> --coop-script <file> --coop-out <file>` runs a game headless from an input script and
  writes its fingerprint every frame. The same script must always give the same file.
- `tools/coop-lab/lab.py` runs 2-4 headless players on one PC, through a local relay with fake lag or directly.
- `tools/coop-lab/windows_lab.py` runs two real windows and checks every frame reaches the screen.
- `COOP_TEST_SWF=<swf> cargo test -p ruffle_desktop --release` runs the session tests.

## Limits

- Up to 4 players. No joining a game that has already started.
- One shared view for now (a camera for each player is being built).
- The game runs at the speed of the slowest PC, so a very slow frame on one PC stalls everyone.
- If anyone leaves, the game ends for everyone.

## Building

Ruffle Together is on the [`coop` branch](https://github.com/multiplayereverything/ruffle-coop/tree/coop).

```
cargo build -p ruffle_desktop --release
```

See [Ruffle's build instructions](https://github.com/ruffle-rs/ruffle#building-from-source) for the
prerequisites.

## Docs

- [User guide](https://github.com/multiplayereverything/ruffle-coop/blob/coop/docs/ruffle-together.md): playing,
  key maps, the co-op API, testing, known limits
- [Design](https://github.com/multiplayereverything/ruffle-coop/blob/coop/docs/superpowers/specs/2026-10-28-ruffle-together-design.md)

## License and credits

Built on [Ruffle](https://github.com/ruffle-rs/ruffle) by the Ruffle team and contributors. Like Ruffle, it is
licensed under either the Apache License, Version 2.0, or the MIT license, at your option. See
[LICENSE.md](LICENSE.md).
