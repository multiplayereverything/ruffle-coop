# Ruffle Together: design

Date: 2026-10-28. Fork: https://github.com/multiplayereverything/ruffle-coop (clean copy of ruffle-rs/ruffle, no
changes yet). First game: Bad Ice-Cream.

## Why

Friends complain about input delay in the Flash online mods, both through the central relay and over Radmin (Radmin
shows 10 ms, the game feels much worse). Measured on 2026-10-28 (global-mods-server `tools/latency_check.py`, MODLOG):

- The relay adds ~0 ms. Nagle on the game's socket adds ~50 ms per message (Ruffle desktop never calls
  `set_nodelay`; Flash Player 32 is the same).
- The current mods are host-authoritative: the friend's key goes to the host, the host runs the game and sends state
  back. The friend feels a full round trip plus Flash's once-a-frame socket reads plus 2-5 frames of buffering
  (Gun Mayhem 2 measured 0.35-0.45 s). Every game also needs its own hand-written mod.

GMTogether (yal.cc/docs/gm/gmtogether) solves this for GameMaker games with lockstep: both PCs run the same game, only
inputs are exchanged, and both players get the same small input delay. Its license forbids taking it apart; we use the
idea only. Ruffle Together is the same idea inside our own Ruffle build.

## Goals

1. Two PCs play a **local 2-player** Flash game together online in our Ruffle build. The game file may be patched the
   way the current mods are (flash-games repo), but only to make it run outside its portal; syncing is all in Ruffle.
   First game: Bad Ice-Cream (Nitrome 2010, AS3, SWF 9, 550x550, 25 fps; P1 arrows + Space/Enter, P2 WASD + F).
2. Both players get the same, small input delay: a few frames, chosen by the host.
3. No sync problems (the user's main requirement): the two copies must stay identical. Prevent drift by design, and
   if it ever happens, detect it quickly and **warn both players** (user's choice: a warning, not an automatic
   restart).
4. Nagle off for every socket in our Ruffle build, so the existing relay mods get ~50 ms better too.

## Non-goals (v1)

- More than 2 PCs **in v1**. The user wants up to 4 later (2026-10-28), so v1 must not paint itself into a corner:
  inputs, fingerprints and the handshake carry a player index and player count, and a frame waits for "all players",
  not "the other player". Only the menus, tests and key mapping are limited to 2 in v1.
- Rollback or hiding delay by predicting. Ruffle can't save/restore a moment of the game, so v1 is plain lockstep.
- Healing a desync mid-level. v1 warns; the players restart the level themselves.
- Web/extension Ruffle, Mac, Linux, Steam Deck. Windows desktop only.
- Games without local multiplayer, and replacing the existing per-game relay mods. Those keep working.

## Anti-goals (a failure even if it "works")

- Games that drift out of sync without anyone being told.
- Breaking normal single-player Ruffle: with no session, the build must behave exactly like upstream Ruffle.
- Putting online code in the game file. The sync must not depend on per-game patches.

## How it works

**Game file (user's decision):** as the current mods do, a patched copy in the flash-games repo. For Bad Ice-Cream that
is the Poki build with **only** the Poki fix (levels start without Poki's ad callback; Poki JavaScript calls do
nothing), made with the existing mod's `mod/tools/make_poki_src.py` route, and without the current mod's online code
(`Net`, F9 panel), which would fight Ruffle Together.

Both PCs load the same SWF (checked by hash before starting). Each frame number N runs on both PCs with the same
inputs:

1. Each PC reads its own keyboard for frame N and sends it, tagged for frame N + D (D = input delay in frames).
2. A PC runs frame N only when it holds both players' inputs for N. Until then it waits (the window keeps drawing;
   if a peer is late for more than ~2 s, show "waiting for <name>").
3. Before running frame N, the inputs for N are fed into Ruffle as key events: the host's keys as Player 1, the
   friend's keys as Player 2. Each player uses their own normal keys (arrows or WASD, Space/Enter/F); our build maps
   them to the game's P1 or P2 keys. The mapping is a small per-game table (Bad Ice-Cream: P1 = arrows/Space/Enter,
   P2 = WASD/F).
4. Mouse (my call): **only the host's mouse reaches the game**, as position + button in stage coordinates, sent
   every frame like keys. The friend's mouse is ignored by the game and the friend sees the host's cursor. Flash has
   one mouse, so two players moving it would fight; and the host already picks levels in today's Bad Ice-Cream mod.
   Gameplay is keyboard only.

### Sources of drift and how each is removed

Found in Ruffle's source (2026-10-28):

| Source | Where in Ruffle | v1 change |
|---|---|---|
| Random numbers (Bad Ice-Cream uses Math.random for every enemy walk, chaser target, teleports) | `core/src/avm_rng.rs`: one `AvmRng`, seeded from the wall clock in `get_seed` | Seed from a value the host picks and sends at start |
| getTimer | `core/src/avm2/globals/flash/utils.rs` `get_timer`: `Instant::now()` since start | Return frame count × frame duration |
| Date / clock | locale backend `get_current_date_time` | Return a fixed start time + frame count × frame duration |
| Frame pacing | `core/src/player.rs` `tick`: wall-clock accumulator, `max_frames_per_tick` catch-up, audio-skew adjustment of the accumulator | In a session: run exactly one frame when the lockstep loop says so (Ruffle already has stepping: `suspend_after_next_frame`); no catch-up, no audio skew |
| Timers (setTimeout/setInterval, flash.utils.Timer) | `update_timers(dt)` with real dt | Advance by exactly one frame duration per frame |
| Socket/NetConnection/URL loads finishing at different times | `update_sockets`, loaders | Not needed by Bad Ice-Cream gameplay; v1 blocks network loads in a session (the Poki build may try to reach Poki; test the copy we use) |
| Window focus, resize, fullscreen | events from the frontend | Not forwarded as game events in a session; the game always sees "focused", fixed stage size |
| Save data (SharedObject `so_badicecream`: unlocks) | shared object storage | Host's save is sent to the friend at start and used for the session; the friend's own save is untouched |
| Sound | audio backend | Sound never feeds back into game state in v1 (no audio-skew); sound itself may differ |

### Sync check (detect)

Every 25 frames (1 s at 25 fps) each PC computes a fingerprint of the game: frame number, RNG state, and the
positions/frames of all display objects on stage. They exchange fingerprints. On the first mismatch, both show a
warning at the same frame ("The two games are out of sync - restart the level") and log both fingerprints plus a
screenshot, so we can find the cause. The game keeps running; the players decide.

### Connecting

Host / Join from a small Ruffle Together menu (before the game loads):

- **Direct:** the host listens on a TCP port; the friend types the host's IP (works over Radmin, LAN, or a forwarded
  port).
- **Room code:** through the central relay at 195.60.166.57:25569 as a new star-mode game id (`ruffle-coop`,
  2 players), with the code shown on the host. Needs a mods-info file for the new id (user's OK before pushing).

Both use TCP with Nagle off (my call: our relay and Radmin both already speak TCP, and at 25 fps a frame is 40 ms, so
TCP is fine for v1; UDP and direct peer-to-peer can come later if measurements show a need).

The handshake checks: same Ruffle Together build, same SWF hash, then the host sends the RNG seed, input delay,
start time, and save data.

Getting the build (my call): a Windows zip published as a GitHub release on the fork. A mods-info file for
`ruffle-coop` carries the latest version and the release link, so the menu can say "update available" (pushed only
with the user's OK). Rooms on the relay use the v2 handshake's exact build match (`version#hash`), so a friend on a
different build is refused at join with a clear message instead of desyncing later.

### Input delay

The host sets D in frames (default 3 = 120 ms at 25 fps; my call, to be tuned by measurement). The menu shows the
measured round trip so the host can pick D ≈ round trip / frame time + 1.

## Success looks like

- Two PCs (or two copies on one PC through the relay) finish several Bad Ice-Cream levels together with the sync
  check never firing.
- Felt input delay over Radmin is D frames for both players, and lower than the current Bad Ice-Cream relay mod
  (measured with a scripted key press -> on-screen move test on both PCs).
- Single-player Ruffle with no session behaves like upstream (upstream test suite still passes).

## Testing approach (builder's choice of details)

- Two copies on one PC, scripted inputs (no taking over the user's keyboard or mouse), small test windows.
- Determinism test without networking first: run the same recorded input stream twice and compare fingerprints frame by
  frame. This is the main guard against sync problems and must pass before networking is added.
- Then lockstep over a local relay on a free port (never 7777/7778/27015/27020), with added lag/jitter.
- Two real PCs last, by the user.

## Left to the builder

- Exact patch points and code structure inside the fork; keeping the diff small so upstream Ruffle can be merged later.
- Wire format of inputs/fingerprints.
- Which display-object fields go into the fingerprint.
- The look of the Ruffle Together menu and warnings.

## Open questions

- None blocking v1. Later: more than 2 PCs, UDP/peer-to-peer, rollback, other games (Gun Mayhem 2, Raft Wars,
  Territory War all have local multiplayer).
