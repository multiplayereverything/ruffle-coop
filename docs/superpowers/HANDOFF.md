# Ruffle Together hand-over (2026-10-29)

Owner: the "Custom Ruffle build" session (Ruffle Together = lockstep co-op Ruffle fork). Read first:
`docs/ruffle-together.md` (play, key maps, co-op API, testing, limits), `docs/superpowers/coop-log.md` (results, newest
at the end), `docs/superpowers/coop-decisions.md`. Game-builder spec sent to every session:
`C:\Users\liam1\AppData\Local\Claude-Data\claude_pzs8sxrjxfjjc\side-session-notes\local_de448b6c-9292-4052-a6e3-594bb10fbc31\ruffle-together-api-and-keymaps.md`.

## State
- Released: **v1.0.3.3** (tag on 4a2da85; mods-info ruffle-coop 1.0.3.3; launcher 1.0.34 auto-updates). Branch `coop`
  pushed to 5c7a11e. **Local, unpushed:** e7db153 Territory War keys, d8dd27d co-op API, 9f4ae0d key-map files, b70178a log.
- Unreleased work (all tested, 107/107): co-op API (`coop.players/localSlot/mouseX/mouseY/mouseDown(slot)` via
  ExternalInterface, AS2=AS3; `desktop/src/coop/api.rs`, installed by `Stepper` in `frame.rs`); every PC sends its mouse;
  scripts `move/click X Y [slot]`; buttons KEY1-4 VIEW QUIT YES NO (`lockstep.rs` `button_named`); key maps are text files
  (`keymap.rs`, `desktop/src/coop/keys/*.keys`, `--coop-keys <name|file>`, handshake compares name#sha256); built in:
  bad-ice-cream, gun-mayhem-2, territory-war, bow-man, smileys-war, stealth-assassin, stealth-hunter-2, intrusion,
  intrusion-2, air-battle. Core: `coop_key_codes` (core/src/input.rs) checks the key-code table.
- Test build for game sessions: `test-build\ruffle_desktop.exe` + `*.keys` (git-ignored). `target\release\ruffle_desktop.exe`
  is usually LOCKED by other sessions' lab processes: cargo builds `target\release\deps\ruffle_desktop.exe` anyway
  (NOT `deps\ruffle_desktop-<hash>.exe`, that's the test runner). Copy that one.

## Next steps
1. **Ask the user** to release 1.0.4 (API + 10 maps). Release recipe = coop-log entries for 1.0.3.x: bump VERSION in
   `desktop/src/coop/session.rs` (+ build_id test) and line 3 of docs/ruffle-together.md; full tests; commit; push; build
   (exe from deps if locked; check the commit sha is in the exe); zip ruffle_desktop.exe + README.md (=docs/ruffle-together.md)
   + LICENSE.md as `RuffleTogether-<v>-windows-x64.zip`; tag; `gh release create ... --latest`; re-download + hash; live
   relay check; bump mods-info in a FRESH clone (scratchpad; `global-mods-server/mods-info` has others' uncommitted work):
   latest/min/protocol `rc-<v>`, links, README row; `python -I tools/validate.py`; push; log. Then tell every session below.
2. **Ultimate Crab Battle** (local_3ae3db7a…) asked for `ultimate-crab-battle` — NOT done. Its layout: P1 arrows + FIRE A(65);
   P2 I/J/K/L 73/74/75/76 + U(85); P3 T/F/G/H 84/70/71/72 + R(82); P4 numpad 8/4/5/6 + 0 (104/100/101/102/96). Every slot also
   presses KEY1-3 = 49-51, ALT = Ctrl 17, YES = Y 89, NO = N 78 (put them in every slot line; not `shared`, since movement is
   per slot). Local: arrows/W S D move (no A: it's FIRE in game? it asked FIRE=Space,Enter,Z,F), ALT=17, KEY1-3=49-51,
   YES=89, NO=78. Note: KEY digits need the character ('1') — key_descriptor already sends Character for 48-57.
   Add the .keys file + BUILT_IN entry + a test; reply with the build.
3. **Territory War** (local_e42aa7c6…, handing off to a fresh session) reports windows_lab FAIL on late frames at 40 fps
   (in sync; 28/16 late quiet). The limit is 1.25 frame times (31 ms at 40 fps) and the PC was busy. Investigate: re-run
   alone with `--exe test-build\ruffle_desktop.exe --script "...\Territory War\mod\coop\territory-war-2p.txt" --keys
   territory-war --seconds 120`; compare frame cost/late lines with BIC/GM2; decide whether the limit or the build is wrong.
   It releases its SWF to flash-games only after 1.0.4 ships.
4. Undelivered: **Bow Man** (local_b466301b…) reply failed ("workspace requires trust approval"): its map + API are in the
   spec. Queued: mod-folders parent, Stealth Hunter 2, Raft Wars (API spec). Raft Wars uses gun-mayhem-2 (no new map).
5. Not supported (said so): per-PC camera/views, >4 players, mid-game joins.
6. Confirmed by the Ultimate Assassin session (local_f53b835c…): `stealth-assassin` works on the test build (2p replay
   = gun-mayhem-2's fingerprints; `p0 KEY1` = Space pauses). It will send .keys files for Ultimate Assassin 3, 2, 1:
   add each as a built-in (file + BUILT_IN entry) for the next release.

## Build / test
- `cargo build -p ruffle_desktop --release`; tests: `COOP_TEST_SWF=<BadIceCream_coop.swf> cargo test -p ruffle_desktop --release -- --test-threads=1`.
- Labs: `tools/coop-lab/lab.py --exe --swf --script --keys --players --mode` (headless sessions), `tools/coop-lab/windows_lab.py`
  (two real windows; limits follow the game's fps). Replay: `--coop-replay <swf> --coop-keys <m> --coop-script <s> --coop-out f`.
- Baselines (must stay equal unless keys change): BIC fa623659, GM2 4p 8e30f66c (script gm2-mod/ruffle-together/ruffle-together-4p.txt), TW 8f0d87b8.

## Traps
- Write patch scripts with the Write tool (bash heredocs mangle `\n`). LF files in this repo.
- User rules: ask before push/release/mods-info/flash-games; never drive keyboard/mouse (scripts; windows only for tests,
  capture with PrintWindow, close only your own PIDs); avoid ports 6420, 7777-7779, 27015/16, 27020/21; results in a table.
- localSlot must never move/show/hide/create display objects: the fingerprint hashes position/visibility/depth/frame.
- Opening a Ruffle window closes the previous one per cache dir: give test windows their own `--cache-directory`.
