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
