# Ruffle Together Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two PCs play a local-multiplayer Flash game (first: Bad Ice-Cream) together online in our Ruffle build, using lockstep input sync that never drifts silently.

**Architecture:** Both PCs run the same SWF. Each frame runs only when every player's input for it has arrived; inputs are injected as key/mouse events, and everything in Ruffle that reads the real clock (getTimer, Date, RNG seed, timers, frame pacing) is driven by the frame count instead. A fingerprint of the game is compared every 25 frames; a mismatch shows a warning on both PCs. Core gets small, opt-in hooks (no behavior change without a session); the session logic lives in a new `desktop/src/coop/` module.

**Tech Stack:** Rust (Ruffle workspace, stable toolchain from `rust-toolchain`/CI), egui (existing desktop GUI), std TCP threads, the existing Python relay (`global-mods-server/init.py`) for room codes.

**Spec:** `docs/superpowers/specs/2026-10-28-ruffle-together-design.md` (copy of `global-mods-server/docs/2026-10-28-ruffle-together-design.md`, approved by the user 2026-10-28).

## Global Constraints

- Windows desktop only (`desktop` crate). Web/extension untouched.
- With no session, the build behaves exactly like upstream Ruffle: every core hook defaults to off; upstream tests (`cargo test -p ruffle_core`, `cargo test -p tests`) still pass.
- v1 plays 2 PCs, but every message, the lockstep queue and the handshake carry `player_index` and `player_count`, and frames wait for "all players". Up to **4** must work later without a rewrite.
- Input delay default **3 frames**, host-set; Bad Ice-Cream runs at **25 fps** (frame = 40 ms).
- Sync check every **25 frames**. On mismatch: **warn both players, keep running** (user's choice). No automatic restart.
- Only the **host's mouse** reaches the game; gameplay input is keyboard.
- Transport: TCP with `TCP_NODELAY`. Direct (host listens, friend types IP; works over Radmin) and relay room code via `195.60.166.57:25569`, game id **`ruffle-coop`**, `RELAY|2` handshake with version `"<build version>#<build hash>"` (exact match on join).
- Game file: a patched Bad Ice-Cream with **only** the Poki fix (no online code), built by the Bad Ice-Cream mod's `build.py`.
- Never use ports 7777, 7778, 7779, 27015, 27016, 27020, 27021, 6420 for tests. Never take over the user's keyboard or mouse in tests: scripted input only, small windows.
- Work on branch `coop` in this fork. Commit locally per task; pushing to GitHub, publishing a release and pushing mods-info each need the user's OK.

## Review Focus

1. A peer disconnects or crashes mid-level → the other side must stop waiting within ~5 s and show "<name> left", not freeze forever. (Task 7, Task 10)
2. One window is minimized or loses focus → frames must keep running (no stall for the other player), and keys held at that moment must be released, not stuck. (Task 8, Task 10)
3. Network burst: inputs for several frames arrive at once after a stall → the late PC runs the backlog quickly (several frames per loop) instead of staying behind forever. (Task 5, Task 10)
4. Mismatched game file or build → refused at the handshake with a clear message naming which one differs, never a silent desync. (Task 6, Task 7)
5. The SWF changes its frame rate mid-game (`stage.frameRate`) → both PCs must use the frame rate in effect for that frame for the clock and timers. (Task 2, Task 10)

---

### Task 0: Branch, toolchain and baseline

**Files:** none changed.

- [ ] **Step 1:** `git checkout -b coop` in `C:\Users\liam1\Desktop\AIStuff\ruffle-coop` (unshallow first: `git fetch --unshallow`).
- [ ] **Step 2:** Build: `cargo build -p ruffle_desktop --release`. Expected: `target/release/ruffle_desktop.exe` exists. If the toolchain is missing, stop and tell the user what to install (rustup, MSVC build tools); don't install system software without their OK.
- [ ] **Step 3:** Baseline tests: `cargo test -p ruffle_core` and `cargo test -p ruffle_frontend_utils`. Record pass counts in `docs/superpowers/coop-log.md` (create it; it is this project's MODLOG).
- [ ] **Step 4:** Copy the spec to `docs/superpowers/specs/2026-10-28-ruffle-together-design.md`. Commit: `docs: Ruffle Together design and plan`.

### Task 1: Nagle off for game sockets

**Files:**
- Modify: `frontend-utils/src/backends/navigator.rs` (the `TcpStream::connect` in `connect_socket`, ~line 362)
- Test: same file's `#[cfg(test)]` module (it already has `start_test_server`)

**Interfaces:** Produces: `async fn connect_tcp(host: &str, port: u16) -> io::Result<TcpStream>` (module-private), which connects and calls `set_nodelay(true)`.

- [ ] **Step 1: Failing test** `test_socket_nodelay`: start the test server, `let s = connect_tcp("127.0.0.1", addr.port()).await.unwrap(); assert!(s.nodelay().unwrap());`
- [ ] **Step 2:** Run `cargo test -p ruffle_frontend_utils test_socket_nodelay`. Expected: FAIL (function missing).
- [ ] **Step 3:** Add `connect_tcp`; use it in `connect_socket`. A `set_nodelay` error is logged with `warn!` and ignored.
- [ ] **Step 4:** Run the navigator tests. Expected: all PASS.
- [ ] **Step 5:** Commit: `desktop: disable Nagle on Flash sockets`. (Its effect on the existing relay mods is measured in Task 12, Step 4, by running the current Bad Ice-Cream relay mod in this build and in upstream Ruffle.)

### Task 2: Frame clock in core (getTimer, Date, timers)

**Files:**
- Create: `core/src/frame_clock.rs`
- Modify: `core/src/lib.rs` (module), `core/src/context.rs` (field), `core/src/player.rs` (field, builder, setter, pass into context), `core/src/avm2/globals/flash/utils.rs` (`get_timer`), `core/src/avm1/activation.rs` (~line 1259, AVM1 getTimer)
- Test: `core/src/frame_clock.rs` unit tests

**Interfaces:**
- Produces:
  - `pub struct FrameClock { start_unix_ms: i64, elapsed: Duration }` with `pub fn new(start_unix_ms: i64) -> Self`, `pub fn advance(&mut self, frame: Duration)`, `pub fn elapsed(&self) -> Duration`, `pub fn now_utc(&self) -> chrono::DateTime<Utc>`.
  - `Player::set_frame_clock(&mut self, clock: Option<FrameClock>)` and `Player::advance_frame_clock(&mut self, frame: Duration)` (no-op when None).
  - `UpdateContext::elapsed_since_start(&self) -> Duration`: the frame clock's `elapsed()` when set, otherwise `Instant::now().duration_since(start_time)` (today's behavior).
  - `pub struct FrameClockLocale { clock: Arc<Mutex<FrameClock>> }` implementing `LocaleBackend` (`get_current_date_time` = `now_utc()`, timezone fixed UTC+0) so `Date` and the RNG seed (`avm_rng::get_seed` reads the locale) follow the frame clock.

- [ ] **Step 1: Failing tests** in `frame_clock.rs`: `advance_adds_exact_frames` (new(1_700_000_000_000); advance 40 ms ×25 → `elapsed() == 1000 ms`, `now_utc().timestamp_millis() == 1_700_000_001_000`); `locale_follows_clock` (FrameClockLocale sees the advanced time).
- [ ] **Step 2:** `cargo test -p ruffle_core frame_clock`. Expected: FAIL.
- [ ] **Step 3:** Implement; switch both getTimer sites to `elapsed_since_start()`.
- [ ] **Step 4:** Tests PASS; `cargo test -p ruffle_core` still at the Task 0 baseline.
- [ ] **Step 5:** Commit: `core: optional frame clock for getTimer, Date and RNG seed`.

### Task 3: Game fingerprint in core

**Files:**
- Create: `core/src/coop_fingerprint.rs`
- Modify: `core/src/avm_rng.rs` (add `pub fn state(&self) -> u32`), `core/src/player.rs` (`pub fn coop_fingerprint(&mut self) -> u64`), `core/src/lib.rs`
- Test: unit test for `AvmRng::state`; the fingerprint itself is proven in Task 4.

**Interfaces:** Produces `Player::coop_fingerprint(&mut self) -> u64`: a stable hash (FNV-1a 64, no `std` RandomState) of: the frame clock's elapsed ms, `AvmRng::state()`, and a depth-first walk of the stage's display list with, per object: depth, a type tag (character id, or AS3 class name for code-made objects), x and y in twips, current frame (movie clips), visible flag.

- [ ] **Step 1: Failing test** `rng_state_reports_u_value`: a fresh `AvmRng` reports 0; after `generate_random_number`, `state() != 0`.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Implement `state()` and `coop_fingerprint` (walk via the existing `DisplayObject`/`TDisplayObjectContainer` iteration used by the debug tools).
- [ ] **Step 4:** Tests PASS. Commit: `core: game fingerprint for sync checks`.

### Task 4: Determinism checker (`coop-replay`)

The main guard against sync problems; must pass before any networking.

**Files:**
- Create: `tools/coop-replay/Cargo.toml`, `tools/coop-replay/src/main.rs`; add `"tools/coop-replay"` to the workspace `members`.
- Test fixtures (not committed; the game file is not ours): `tools/coop-replay/local/` in `.gitignore`.

**Interfaces:**
- Consumes: Task 2 `FrameClock`, `FrameClockLocale`, `Player::set_frame_clock/advance_frame_clock`; Task 3 `coop_fingerprint`; `ruffle_input_format::InputInjector` (existing JSON input format: `KeyDown`/`KeyUp`/`MouseMove`/`MouseDown`/`Wait`).
- Produces: CLI `coop-replay <game.swf> <inputs.json> --frames N [--jitter-ms M] [--no-clock] --out fingerprints.txt`, writing one `frame fingerprint` line per frame. Headless player (no renderer/audio backends, null navigator that refuses loads, in-memory storage), stepping exactly like `tests/framework/src/runner.rs` `do_tick`: inject that frame's events, `run_frame()`, `update_timers(frame_time)`, `advance_frame_clock(frame_time)`. `--jitter-ms` sleeps a random 0..M ms between frames (proves wall-clock speed doesn't matter). `--no-clock` skips the frame clock (upstream behavior; the negative control).

- [ ] **Step 1:** Write `local/bic-two-players.json`: from the title screen, the inputs (host mouse clicks in stage coordinates) to start a 2-player game and play ~60 s with both players' keys (arrows/Space and WASD/F) pressed in a fixed pattern. Find click positions from a screenshot of `--frames` output or the mod's notes.
- [ ] **Step 2:** Run twice with `--jitter-ms 0` and `--jitter-ms 30` on the Poki-fix-only SWF from Task 11 (until Task 11 is done, use the mod's existing `dist` build only to develop the tool). Expected: the two `fingerprints.txt` files are **identical** for all 1500 frames.
- [ ] **Step 3: Negative control:** run twice with `--no-clock --jitter-ms 30`. Expected: files **differ** (random enemies use a clock-based seed). If they don't differ, the fingerprint is too weak: add fields until it does.
- [ ] **Step 4:** Record both results in coop-log. Commit: `tools: coop-replay determinism checker`.

### Task 5: Lockstep queue

**Files:** Create `desktop/src/coop/mod.rs`, `desktop/src/coop/lockstep.rs`; modify `desktop/src/main.rs` (`mod coop;`). Tests in `lockstep.rs`.

**Interfaces:** Produces:
- `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] pub struct InputFrame { pub buttons: u16, pub mouse: Option<(i32, i32, bool)> }` — `buttons` bits: `UP=1, DOWN=2, LEFT=4, RIGHT=8, FIRE=16, ALT=32` (constants in `lockstep.rs`); mouse in stage twips, host only.
- `pub struct Lockstep` with `new(player_count: u8, local_index: u8, delay: u32) -> Self`, `push_local(&mut self, input: InputFrame) -> (u32, InputFrame)` (stores it for `current_frame + delay`, returns the tagged frame to send), `push_remote(&mut self, player: u8, frame: u32, input: InputFrame)`, `ready(&self) -> Option<Vec<InputFrame>>` (all players' inputs for the next frame, by player index), `advance(&mut self)`, `frame(&self) -> u32`, `waiting_on(&self) -> Vec<u8>`.
- Frames `0..delay` are pre-filled with empty input for everyone so the game starts immediately.

- [ ] **Step 1: Failing tests:** `waits_for_all_players` (2 players, only local pushed → `ready()` None after the prefill; push remote → Some); `delay_tags_future_frame` (delay 3, first `push_local` returns frame 3); `four_players` (player_count 4: needs all four); `burst_catch_up` (push remote inputs for frames 3..10 at once → `ready()`/`advance()` loop runs 8 frames); `waiting_on_names_missing`.
- [ ] **Step 2:** `cargo test -p ruffle_desktop coop::lockstep`. Expected: FAIL.
- [ ] **Step 3:** Implement with a `BTreeMap<u32, [Option<InputFrame>; 4]>`.
- [ ] **Step 4:** PASS. Commit: `coop: lockstep input queue`.

### Task 6: Messages

**Files:** Create `desktop/src/coop/protocol.rs` (+ tests).

**Interfaces:** Produces `pub enum Msg` and `fn encode(&Msg) -> String` / `fn decode(&str) -> Result<Msg, String>`, one text line per message (no `\0`, `|`-separated, fits the relay's star framing):
- `Hello { build: String, swf_sha256: String, name: String }` → `HI|build|sha|name`
- `Start { player_count: u8, your_index: u8, delay: u32, start_unix_ms: i64, frame_rate_milli: u32, save: Vec<u8> }` → `ST|...|<save as hex>`
- `Input { player: u8, frame: u32, input: InputFrame }` → `IN|p|f|buttons|mx|my|mb` (`-` for no mouse)
- `Print { player: u8, frame: u32, hash: u64 }` → `FP|p|f|hex`
- `Bye { player: u8, reason: String }` → `BY|p|reason`
- `Refuse { reason: String }` → `NO|reason` (build or SWF mismatch; reason names which).
Names and reasons are cleaned of `|` and control characters.

- [ ] **Step 1: Failing tests:** round trip for every variant; `decode("IN|0|x|...")` is `Err`; a name containing `|` survives as a cleaned name.
- [ ] **Step 2:** Run; FAIL. **Step 3:** Implement. **Step 4:** PASS. **Step 5:** Commit: `coop: wire messages`.

### Task 7: Connections (direct and relay)

**Files:** Create `desktop/src/coop/net.rs` (+ tests in `desktop/tests/coop_net.rs`).

**Interfaces:**
- Consumes: Task 6 `Msg`, `encode`, `decode`.
- Produces: `pub enum Role { HostDirect { port: u16 }, JoinDirect { addr: String }, HostRelay { relay: String }, JoinRelay { relay: String, code: String } }`; `pub fn connect(role: Role, version: &str, name: &str) -> io::Result<Link>`; `pub struct Link` with `send(&self, &Msg)`, `try_recv(&self) -> Option<Event>`, `room_code(&self) -> Option<String>`; `pub enum Event { Msg { from: u8, msg: Msg }, Joined { player: u8, name: String }, Left { player: u8 }, Closed(String) }`. Reader/writer threads, `TCP_NODELAY`, lines framed by `\0` (relay star framing) in both modes.
- Relay: `RELAY|2|HOST|ruffle-coop|<version>|0|2||<room name>` / `RELAY|2|JOIN|ruffle-coop|<version>|<code>||<name>`; guest messages arrive at the host as `G|<id>|<line>`, `J|`/`L|`/`X` become `Joined`/`Left`/`Closed`. The **host forwards** each guest's `IN` and `FP` lines to the other guests (star mode delivers host→all only), so 4 players work later.
- Direct: the host accepts up to `player_count - 1` connections and plays the same forwarding role.
- A peer silent for 5 s while the game waits → `Left` (Review Focus 1).

- [ ] **Step 1: Failing tests:** `direct_roundtrip` (host on a free port, join, exchange `Hello` both ways); `relay_roundtrip` (start `python <global-mods-server>/init.py --port <free> --no-mods-info --games <tmp games.json with {"ruffle-coop": {"mode":"star","max_players":4}}>`, host gets a code, join with it, exchange `Input`); `relay_version_mismatch` (join with a different `#hash` → `Closed` with the relay's "Version mismatch" text); `peer_silence_is_left` (peer stops sending → `Left` within 6 s).
- [ ] **Step 2:** `cargo test -p ruffle_desktop --test coop_net`. FAIL. **Step 3:** Implement. **Step 4:** PASS. **Step 5:** Commit: `coop: direct and relay connections`.

### Task 8: Keys in, keys out

**Files:** Create `desktop/src/coop/keymap.rs` (+ tests).

**Interfaces:** Produces:
- `pub fn local_buttons(held: &HashSet<PhysicalKey>) -> u16`: arrows or WASD → directions; Space, Enter or F → `FIRE`. Everyone uses their own normal keys.
- `pub struct GameKeys { slots: Vec<SlotKeys> }` with `pub fn bad_ice_cream() -> Self` (slot 0: arrows + Space; slot 1: W A S D + F) and `pub fn events(&mut self, slot: u8, buttons: u16) -> Vec<PlayerEvent>` that emits `KeyDown`/`KeyUp` only for bits that changed since that slot's last call (with full `KeyDescriptor`: physical, logical and location).
- `pub fn release_all(&mut self) -> Vec<PlayerEvent>` (KeyUp for everything held; used when a session ends).

- [ ] **Step 1: Failing tests:** `wasd_and_arrows_both_move` (W → UP, ArrowUp → UP); `slot1_maps_to_wasd` (slot 1 UP|FIRE → KeyDown W and F); `only_changes_emitted` (same buttons twice → no events second time); `release_on_zero`.
- [ ] **Step 2–4:** FAIL → implement → PASS. **Step 5:** Commit: `coop: key mapping`.

Focus loss (Review Focus 2) is handled in Task 10 by treating the local held set as empty while the window is unfocused.

### Task 9: Shared save for the session

**Files:** Create `desktop/src/coop/storage.rs` (+ tests).

**Interfaces:** Produces `pub struct SessionStorage { data: HashMap<String, Vec<u8>> }` implementing `ruffle_core::backend::storage::StorageBackend` (writes stay in memory; the friend's real save is untouched), with `pub fn from_bytes(&[u8]) -> Self`, `pub fn to_bytes(&self) -> Vec<u8>`, and `pub fn export_from_disk(storage_dir: &Path, swf_url: &Url) -> Vec<u8>` that packs the host's shared objects for that SWF (same path rule as the desktop's disk storage backend).

- [ ] **Step 1: Failing tests:** `roundtrip_bytes`; `put_does_not_touch_disk` (temp dir unchanged after `put`).
- [ ] **Step 2–4:** FAIL → implement → PASS. **Step 5:** Commit: `coop: session save data`.

### Task 10: Run the game in lockstep

**Files:** Create `desktop/src/coop/session.rs`; modify `desktop/src/app.rs` (`about_to_wait` ~line 386, `handle_player_event`, focus/key/mouse paths), `desktop/src/player.rs` (player options: locale, storage, navigator for a session), `desktop/src/cli.rs` (flags), `desktop/src/gui/` (warning and waiting banners).

**Interfaces:**
- Consumes: Tasks 2, 3, 5–9.
- Produces: `pub struct Session` with `pub fn tick(&mut self, player: &mut Player) -> SessionState` called from `about_to_wait` **instead of** `player.tick(dt)` while a session exists. Each call: send this frame's local input (`push_local`), drain `Link` events, then `while let Some(inputs) = lockstep.ready()` (cap 10 frames per call): inject `GameKeys::events` for every slot plus the host's mouse, `run_frame()`, `update_timers(frame_time)`, `audio_mut().tick()`, `advance_frame_clock(frame_time)` with `frame_time` from the player's **current** frame rate (Review Focus 5), `lockstep.advance()`; every 25th frame send `Print` and compare stored prints. `SessionState { waiting_on: Vec<String>, desync_at: Option<u32>, closed: Option<String> }`.
- CLI flags (v1 menu): `--coop-host <port>`, `--coop-host-relay <host:port>`, `--coop-join <ip:port>`, `--coop-join-relay <host:port> --coop-code <CODE>`, `--coop-delay <frames>` (default 3), `--coop-name <name>`, and for tests only `--coop-script <inputs.json>` (local input from a file instead of the keyboard, never the user's devices).
- While a session runs: local keyboard events go to `local_buttons`, not to the player; mouse reaches the player only on the host (via the lockstep input); `FocusLost`/`FocusGained` are not forwarded; while unfocused the local held set is treated as empty (Review Focus 2); the loop keeps running when minimized.
- Banners: "Waiting for <name>…" after 2 s with no progress; "<name> left" on `Left`; on desync, both PCs show **"The two games are out of sync — restart the level"** from the mismatching frame on, and write both fingerprints, the frame number and a screenshot to `coop-desync-<time>.txt/.png` next to the log. The game keeps running.

- [ ] **Step 1: Failing test** `desktop/tests/coop_session.rs` `two_sessions_stay_in_sync`: two headless `Session`s over a direct loopback `Link` run the Poki-fix SWF (path from env `COOP_TEST_SWF`, test skipped if unset) with `bic-two-players.json` split into host and friend scripts for 1500 frames; assert no `desync_at` and equal fingerprints at frame 1500.
- [ ] **Step 2: Failing test** `desync_is_reported`: same, but the friend's frame clock starts 1 ms later; assert both report `desync_at = Some(n)` with the same `n`.
- [ ] **Step 3: Failing test** `peer_leaves`: drop the friend's `Link` at frame 500 → host `closed`/`waiting_on` shows the friend left within 6 s.
- [ ] **Step 4:** Implement. **Step 5:** All PASS; upstream tests still at baseline. **Step 6:** Commit: `coop: lockstep sessions in the desktop player`.

### Task 11: Bad Ice-Cream with only the Poki fix

**Files (in `C:\Users\liam1\Desktop\AIStuff\ai-work-flash-games\bad-ice-cream\mod`):** `build.py` (new `--no-net`), `tools/make_poki_src.py` (skip starting `Net` and the F9 panel when asked).

Another Claude session may own this mod: message it before editing, or ask it to make the change.

**Interfaces:** Produces `dist/BadIceCream_coop.swf`: levels start without Poki's ad callback, Poki JavaScript calls do nothing, no `bicnet` classes, no F9 panel. Its SHA-256 is recorded in coop-log.

- [ ] **Step 1:** Build with `python build.py --no-net --name BadIceCream_coop.swf`.
- [ ] **Step 2:** Verify with `coop-replay` (Task 4): a 2-player level starts and runs 1500 frames; no `bicnet` class in the SWF (`ListClasses`).
- [ ] **Step 3:** Commit in that repo only if it is a git repo the user allows; otherwise log it in its MODLOG.

### Task 12: Two copies on one PC, measured

**Files:** Create `tools/coop-lab/lab.py` (in the fork; Python, launches processes only).

- [ ] **Step 1:** `lab.py` starts a local relay (`init.py --port <free> --sim-delay 40 --sim-jitter 10 --games <tmp>`), then two `ruffle_desktop.exe` windows (small, `--coop-script` inputs, host and friend), runs 3 minutes, collects both logs.
- [ ] **Step 2:** Expected: no desync line in either log; both reach the same final frame fingerprint; `waiting` time under 5% of frames.
- [ ] **Step 3:** Input delay: the log records, per local press, the frame it was captured and the frame it was applied; expected exactly `delay` frames (3 = 120 ms) on **both** PCs.
- [ ] **Step 4:** Also run direct mode (no relay) and record the same numbers. Compare with the current Bad Ice-Cream relay mod's felt delay (from its lab notes) in coop-log.
- [ ] **Step 5:** Results table to the user (✅ only for what ran; two real PCs stays ⚠️). Commit: `tools: coop lab`.

### Task 13: Room codes on the live server and release (user's OK needed)

- [ ] **Step 1:** Draft `mods/ruffle-coop.json` for mods-info (star, `max_players` 4, server main, latest/min = the build version, notice empty). Validate with `tools/validate.py`. **Ask the user before pushing.**
- [ ] **Step 2:** After the push, check the live relay: `RELAY|2|HOST|ruffle-coop|...` gets a room; a mismatched `#hash` join is refused.
- [ ] **Step 3:** Package `ruffle_desktop.exe` + README (how to host/join, game file from flash-games) as a zip. **Ask the user before publishing** a GitHub release on the fork.
