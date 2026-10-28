# Ruffle Together log

Plan: docs/superpowers/plans/2026-10-28-ruffle-together.md · Spec: docs/superpowers/specs/2026-10-28-ruffle-together-design.md

## 2026-10-28
- Branch `coop` from upstream 07d3e4306. Release build OK (`cargo build -p ruffle_desktop --release`, 4m55s).
- Baseline: `cargo test -p ruffle_frontend_utils --features navigator` 137/138 on upstream: `test_socket_fail`
  fails on this Windows PC before any change (pre-existing).
- Task 1: Nagle off (`connect_tcp` in frontend-utils navigator). `test_socket_nodelay` RED (missing fn) -> GREEN.
- Task 11 (done early): Bad Ice-Cream `build.py --no-net` -> `BadIceCream_coop.swf`, Poki fix only, 0 bicnet,
  SHA-256 4635ddfc255df0a9682d3fab3b7c2942035a498c1e131cdb062436d5d4593494.
- Tasks 2-9 done (frame clock, fingerprint, lockstep, messages, connections incl. the real relay, keys, save).
  SWF regression suite after the frame clock: 4291 passed, 0 failed.
- Task 4 determinism on Bad Ice-Cream (script desktop/tests/coop/bic-two-players.txt: menus -> 2P -> level 1,
  both players moving/firing): same script at 0 ms and 30 ms random delay per frame -> identical fingerprints for
  all 1900 frames. Different start date (Math.random seed) -> differs from frame 784 (first random use).
- Found: a renderer that can't draw changes the game (title screen at frame 308 swaps a clip for a drawn bitmap),
  so replay and sessions both use the real GPU renderer. Risk to watch: games that read drawn pixels back could
  differ between different GPUs; the sync check would catch that.
- Task 10: sessions run in the desktop player (CLI flags, window title shows room/status). Session tests on the
  real game: in sync for 1000 frames; a forced desync is reported on both PCs at the same frame; a friend
  leaving stops the host within 6 s; a different game file is refused; paced at the game's frame rate.
- Task 12 lab (two windowless players, scripted input, 5400 frames = 3 min each, Bad Ice-Cream at 30 fps,
  input delay 3 frames = 100 ms):
  | mode | desync | host felt delay (median / p95) | friend felt delay |
  |---|---|---|---|
  | local relay, 40 ms one-way lag + 0-10 ms jitter | none | 145 / 154 ms | 113 / 117 ms |
  | direct | none | 121 / 125 ms | 114 / 118 ms |
- Not yet: two real PCs, the windowed session by hand, the relay `ruffle-coop` game in mods-info (needs OK).
- Final review (fresh reviewer): fixed the 2 critical and 7 of 9 important findings, each with a test where one
  is possible (friend's cursor outside the window, exact host mouse, full preload, saves in another folder,
  stray connections, refused-friend retry, player settings in the handshake, window/fullscreen/menu/open
  blocked, no network loads, message boxes). Not fixed, documented: sound end/position feeding back into
  games (Bad Ice-Cream doesn't use it), Dictionary object-key order (upstream). All decisions:
  docs/superpowers/coop-decisions.md.
- After the fixes: desktop 86/86 (with COOP_TEST_SWF), core 129/129, SWF suite 4291 passed / 0 failed,
  lab 900 frames both modes in sync (felt delay ~101 ms direct, 101-113 ms through a laggy relay).
- Known limits for v1: Bad Ice-Cream keys only (P pause, M mute and Esc never reach the game), window title
  is the only status display, the windowed session hasn't been run by hand or on two real PCs yet.
- First real windowed test (two ruffle_desktop windows on this PC, scripted keys): frames never started, the
  session's no-loads navigator also blocked the game file itself (the windowless tests load it differently).
  Fixed with SessionNavigator (only the first fetch goes through; test only_the_game_file_loads...). Then:
  direct, 75 s: both windows identical fingerprints at frames 24, 774, 1524, no warnings. Live relay
  195.60.166.57:25569 with room code Z9ELK, 60 s: identical at 24, 774, 1524.
- mods-info: ruffle-coop pushed (025f6f5, user OK); the live relay accepted it 9 minutes later.
- Release v1.0.0 published (user asked): tag v1.0.0 on 0b9247d, asset RuffleTogether-1.0.0-windows-x64.zip (ruffle_desktop.exe, README.md, LICENSE.md), SHA-256 F1886A7E...6590, re-downloaded and hash checked. Build id rc-1.0.0#0b9247df45. Not a prerelease. mods-info ruffle-coop later updated to 1.0.0 (checked 2026-10-28).
- User's first two-PC test (LAN): in sync, but "freezing a lot, choppy, slow motion"; solo play smooth on the same PCs.
  Added a timing line to the log every 5 s (fps, event-loop gaps, frame cost, time waiting for the other PC's
  input and the longest wait). Two windows on this PC, direct: 30.0 fps, frame cost avg 0.3-6.7 ms, 0 ms waiting.
  Through a local relay with 30 ms + 0-120 ms jitter: 30 fps average but the host waited for input 100% of the
  time (longest 150 ms), the friend ~15% (longest 75 ms). Finding: the host needs the ROUND TRIP under the input
  delay (100 ms) because the friend starts one trip later; the friend only needs the one-way trip. Over that, the
  picture follows packet arrival (freeze, then several frames at once). Nagle is off on every socket (checked).
  Not yet known: what the user's LAN does (Wi-Fi?). Test build with the timing line: test-build/ (not committed),
  sha256 cac226fd...cbb9; asked the user for both PCs' ruffle.log and a ping.
- Release v1.0.1 (user chose "Release v1.0.1" to get the timing build into the launcher): commit d125698 (version
  1.0.1, timing line, docs), desktop tests 88/88 with COOP_TEST_SWF, lab direct 450 frames in sync. Tag v1.0.1, asset
  RuffleTogether-1.0.1-windows-x64.zip SHA-256 4285b186...3c7d (exe 6b280c71...9b75), marked latest; releases/latest
  returns it, re-downloaded and hash checked. Live relay 195.60.166.57:25569 with the downloaded exe, room V49FG, 300
  frames: in sync, felt delay host 134 / friend 100 ms (the round-trip asymmetry again). mods-info 1a33560:
  ruffle-coop latest/min 1.0.1, protocol rc-1.0.1 (pushed from a fresh clone; the global-mods-server/mods-info clone
  has other sessions' uncommitted work and was left alone). Asked the Multiplayer Mods Launcher session to update
  installs that already have 1.0.0 (its installer only fetches when ruffle_desktop.exe is missing).
