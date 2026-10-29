# Ruffle Together: notes for agents

This repository is Ruffle (the Flash Player emulator) plus **Ruffle Together**, a lockstep co-op fork: every
player's PC runs the same Flash game and only inputs travel. Work happens on branch `coop`.

## Read first

1. [docs/coop-api.md](docs/coop-api.md): every co-op API call a game can make (`coop.*` through
   ExternalInterface), which answers are shared or local, and what every game gets without calling anything.
   Keep it up to date whenever `desktop/src/coop/api.rs` or a session feature changes.
2. [docs/ruffle-together.md](docs/ruffle-together.md): the user guide (playing, key maps, testing, limits).
3. [docs/superpowers/HANDOFF.md](docs/superpowers/HANDOFF.md): current state and next steps;
   [docs/superpowers/coop-log.md](docs/superpowers/coop-log.md) (results, newest last) and
   [docs/superpowers/coop-decisions.md](docs/superpowers/coop-decisions.md).
4. `graphify-out/` (when present, not committed): a knowledge graph of the source. For "how does X work"
   questions run `graphify query "<question>"` before reading files; refresh it with `/graphify . --update`.

## Where things are

| Area | Files |
|---|---|
| Co-op API answers | `desktop/src/coop/api.rs` |
| One frame from every player's input | `desktop/src/coop/frame.rs` (`Stepper`) |
| Network session (handshake, inputs, leave, chat, pings) | `desktop/src/coop/session.rs`, `protocol.rs`, `net.rs`, `lockstep.rs` |
| Key maps | `desktop/src/coop/keymap.rs`, `desktop/src/coop/keys/*.keys` |
| Camera, HUD, frame clock, fingerprint (core) | `core/src/coop_view.rs`, `core/src/frame_clock.rs`, `core/src/coop_fingerprint.rs` |
| Window overlays (labels, notices, chat, lobby) | `desktop/src/gui.rs` |
| Discord presence | `desktop/src/coop/discord.rs` |
| Game-side helper library | `tools/coop-api/as2/RuffleTogether.as`, `tools/coop-api/as3/RuffleTogether.as` |
| Labs | `tools/coop-lab/lab.py` (headless players), `tools/coop-lab/windows_lab.py` (two real windows) |
| API demo (every `coop.*` call on screen) | `tools/coop-api/demo/` ([README](tools/coop-api/demo/README.md)) |

## Rules

- **Determinism first.** Anything that can differ between PCs (real clock, timeouts, audio device, window
  size, clipboard, local slot) must never reach the game. New API calls are either shared (the same on every
  PC on the same frame) or only change the picture.
- **Tests:** `COOP_TEST_SWF=<BadIceCream_coop.swf> cargo test -p ruffle_desktop --release -- --test-threads=1`.
  The replay fingerprints must stay the same unless a change explains why: BIC 1500 frames `fa623659`,
  GM2 4 players 3300 frames `8e30f66c`, TW 2 players 3000 frames `ad6fe22d` (see HANDOFF.md for the files).
- After any change to the co-op API or the session, run the API demo ([tools/coop-api/demo/README.md](tools/coop-api/demo/README.md)):
  replay twice, two headless players through the relay, then two real windows. Add every new call to it.
- `target\release\ruffle_desktop.exe` is often locked by other sessions' labs: use
  `target\release\deps\ruffle_desktop.exe`.
- Ask the user before pushing, releasing, changing mods-info or the relay. Never drive the real keyboard or
  mouse (use input scripts). Keep `ruffle-stable/` (known-good builds) untouched.
- Report results in a table.
