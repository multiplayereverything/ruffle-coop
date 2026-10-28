# SDD ledger — plan: docs/superpowers/plans/2026-10-28-ruffle-together.md
Spec: docs/superpowers/specs/2026-10-28-ruffle-together-design.md
Pre-flight:
- T2 produces FrameClock/FrameClockLocale/set_frame_clock/advance_frame_clock -> T4, T10 consume: same names. OK.
- T3 produces Player::coop_fingerprint -> T4, T10. OK.
- T5 InputFrame/Lockstep -> T6 (Input msg), T8 (buttons u16), T10. OK.
- T6 Msg/encode/decode -> T7, T10. OK.
- T4 Step 2 needs T11's SWF: Ruling: build T11 right after T4's tool works (order T4 tool -> T11 -> T4 checks) — spec needs the Poki-fix file for the determinism run — cost if wrong: none, order only.
Task 0: complete (branch coop from 07d3e4306; release build OK; baseline frontend-utils 137/138, test_socket_fail pre-existing on Windows)
Task 0: Ruling: commit docs + Task 1 together — docs were uncommitted at Task 0 — cost if wrong: none
Task 1: Ruling: navigator tests need `--features navigator` (plan omitted it) — tests are feature-gated — cost if wrong: none
Task 1: complete (test_socket_nodelay RED->GREEN; frontend-utils --features navigator 138/139, only pre-existing test_socket_fail)
Task 11: complete early (build.py --no-net, BadIceCream_coop.swf sha 4635ddfc..., 0 bicnet)
Task 7/10: Ruling: desktop is a binary crate, so coop net/session tests are unit tests inside the modules, not desktop/tests/*.rs — integration tests can't see binary modules — cost if wrong: none
Task 2: Ruling: set_frame_clock takes Option<Arc<Mutex<FrameClock>>> (plan: Option<FrameClock>) — the FrameClockLocale must share the same clock so Date/seed follow it — cost if wrong: a signature change
Task 2: Ruling: UpdateContext gets frame_clock_elapsed: Option<Duration> fixed per update — getTimer stays constant within a frame, same on every PC — cost if wrong: none
Task 2: complete (frame_clock tests RED->GREEN; cargo test -p ruffle_core --lib 128/128; SWF suite running)
SWF suite after Task 2: cargo test -p tests -> 4291 passed, 0 failed, 355 ignored
Task 5-9: tests written first for all five; RED observed together (missing types, 20 kinds of E04xx), then implemented
Task 5: complete (commit 14bc6d995; 6 lockstep tests incl. 4 players, burst catch-up, duplicates)
Task 6: complete (a597ef2bf; roundtrip all variants, bad lines, name cleaning)
Task 7: Ruling: peer-silence (Review Focus 1) moves from Link to Session (Task 10) — only the session knows it is waiting; Link reports Left/Closed on disconnect — cost if wrong: a 5 s timer in a different file
Task 7: Ruling: relay host passes friends' lines on only when >1 friend; friends ignore their own echo by player field — keeps 2-player traffic minimal, works for 4 — cost if wrong: duplicate inputs (ignored by Lockstep)
Task 7: complete (8acd51be0; direct + real relay roundtrip, version mismatch refused, friend/host leaving; real relay ran, no skips)
Task 8: complete (397c67168; 6 keymap tests)
Task 9: Ruling: export_from_disk(save_dir, swf_file_name) filters .sol paths containing the game's file name (plan: swf_url) — Ruffle's save path holds the file name; other games' saves never sent — cost if wrong: a save not found (friend sees a fresh save)
Task 9: complete (1057350d8; 4 storage tests)
Tasks 5-9: Ruling: mod.rs in the first commit declares all coop modules, so intermediate commits 14bc6d9..397c671 don't build alone — branch tip builds and tests 68/68 — cost if wrong: bisecting is harder in that range
Task 3: complete (fnv + rng state tests RED->GREEN; core lib 129/129; fingerprint strength proven in Task 4)
Task 4: Ruling: the checker is a `--coop-replay` mode of ruffle_desktop (desktop/src/coop/replay.rs), not a tools/coop-replay crate — it must run the same Stepper/GameKeys code a session runs, which lives in the desktop binary — cost if wrong: none (tools crate could wrap it later)
Task 4: Ruling: input file is our own script format (coop/script.rs: wait/pN/click/move), not ruffle_input_format JSON — the same scripts drive --coop-script players in Task 10/12 — cost if wrong: none
Task 4: Ruling: added coop/frame.rs Stepper (one frame: keys+host mouse, run_frame, update_timers, audio tick, advance clock, render once) shared by replay and session — render() each frame because rendering has game side effects
Task 4: FINDING: NullRenderer vs wgpu renderer diverge at frame 308 (title screen swaps a timeline clip for a code-made object when drawing works). Ruling: replay always uses the offscreen wgpu renderer like a session — cost if wrong: if a game reads drawn pixels back, two PCs with different GPUs could differ; the sync check would catch it
Task 4: Ruling: control run = different start date (Math.random seed) instead of --no-clock — the real-clock run differs at frame 0 only because the clock is hashed, which proves nothing about the game; the seed control diverges at frame 784 (first random use) in 1116/1900 frames
Task 4: added Player::coop_describe (readable per-object dump) for desync reports
Task 4: complete (desktop/tests/coop/bic-two-players.txt; replay x2 at 0 and 30 ms jitter IDENTICAL 1900/1900 frames; seed control DIFFERS; automated test bad_ice_cream_runs_the_same_every_time PASS with COOP_TEST_SWF; cargo test -p ruffle_desktop --release coop:: 31/31; core lib 129/129)
Task 10: Ruling: session status shows in the window title (room code, player, waiting/left/OUT OF SYNC), not egui banners — v1, smallest change — cost if wrong: less visible warning; a GUI banner is a follow-up
Task 10: Ruling: the handshake runs in main before the window opens (CLI flags are the v1 menu, as planned) — the friend needs the host's start date and save before the player is built — cost if wrong: no window while the host waits; a GUI menu is a follow-up
Task 10: Ruling: Player::viewport_to_stage/stage_to_viewport added; inputs carry stage twips so different window sizes click the same spot
Task 10: Ruling: added pacing (SessionOpts.paced, frames due by this PC's clock, catch-up max 10/tick) — the plan missed it; without it the game ran unthrottled — cost if wrong: none
Task 10: BUG found by the tests: host used the Start message's your_index (the friend's) for itself -> both played as player 2 and waited forever. Fixed (host is always 0).
Task 10: complete (session tests 5/5 with COOP_TEST_SWF: in sync 1000 frames, desync reported on both PCs at the same frame, friend leaving -> host stops waiting < 6 s, different game file refused, paced 100 frames ~4 s; cargo test -p ruffle_desktop --release 80/80; windowed path built but only exercised by hand: user check)
Task 12: Ruling: the lab runs two windowless players (--coop-headless) instead of two ruffle_desktop windows — the user's rule: never take over the PC; the headless player runs the same Session/Stepper code — cost if wrong: the windowed glue (app.rs input/tick, title) is only checked by hand
Task 12: Ruling: Bad Ice-Cream runs at 30 fps here (not 25 as the mod notes said): 3 frames = 100 ms
Task 12: complete (lab 5400 frames = 180.0 s each mode, no desync, nobody left. Relay with 40 ms one-way lag + 0-10 jitter: host felt delay median 145 / p95 154 ms, friend 113 / 117. Direct: host 121 / 125, friend 114 / 118. Max ~310-380 ms (start-up). Nagle effect on the old relay mods: not measured (needs the mod's windowed lab))
Clippy: my 3 warnings fixed; remaining 2 are upstream (desktop/src/gui/theme.rs unused async error on Windows, frontend-utils fs_helper needless return)
Task 13: not started — needs the user's OK (mods-info push, release)
Final review (fresh reviewer, opus): 2 Critical, 9 Important, 10 Minor. Re-graded by effect; all Critical/Important stand except as ruled below.
Final: fixed C1 friend's cursor outside window dropped host clicks + I3 window transform rounding — host_mouse_is_the_same_on_every_pc RED (mouse_in_stage false) -> GREEN (Stepper sets mouse_in_stage, core stage_mouse_override), suite 38/38
Final: fixed C2 wall-clock streaming preload — first_step_loads_the_whole_movie (Stepper preloads fully on first step); Ruling: the test was GREEN before the fix because Bad Ice-Cream loads within one frame — it guards bigger SWFs — cost if wrong: none (fix is unconditional)
Final: fixed I8 save keys hold the folder — friend_in_another_folder_finds_the_hosts_save RED->GREEN, storage 5/5
Final: fixed I7 stray/extra connections — stray_connection_during_the_game_is_ignored RED ("Player 3 left") and refused_friend_can_retry_with_the_right_file RED (Lockstep assert crash) -> GREEN; suite 41/41
Final: fixed I9 player settings not compared — different_player_settings_are_refused_by_name RED->GREEN; suite 42/42
Final: fixed I4 window/GUI actions (forced SHOW_ALL, no fullscreen/menu/open/drop/close in a session), I5 async loads (NullNavigator in a session), I11 invisible errors/room code (rfd message boxes). Ruling: no automated test — app.rs window glue has no test harness and the lab must not open windows on the user's PC — cost if wrong: a glue mistake shows up only in the user's hand test
Final: Ruling: I6 sound feeding back (soundComplete/position) not fixed — Bad Ice-Cream uses neither (checked its decompiled code); a frame-clocked audio position is a larger change — cost if wrong: another game gated on sound ends desyncs, caught by the sync check
Final: Ruling: I10 Dictionary object-key order is upstream behavior; Bad Ice-Cream doesn't iterate object-keyed Dictionaries — documented as a limit — cost if wrong: such a game desyncs, caught by the sync check
Final: minor (deferred): right/middle click counts as left click; mouse button held on focus loss not released
Final: minor (deferred): give-up close sends no Bye; friend sees "the host left" not the host's name; window drag may pause the loop >5 s
Final: minor (deferred): last_frame_at starts at Session::start, a slow window start could trip the 5 s give-up; host can't cancel waiting without closing
Final: minor (deferred): Stepper never runs tick(): NetStream video and sockets freeze in a session; AVM1 time_offset not reset (deterministic)
Final: minor (deferred): desync report has no screenshot
Final: minor (deferred): poisoned session mutex falls back to normal ticking
Final: minor (deferred): fingerprint leaves out scale/rotation/alpha/text/variables
Final: minor (deferred): flash.crypto.generateRandomBytes not deterministic
Final: minor (deferred): GameKeys hard-coded to Bad Ice-Cream; Esc/P/M never reach the game
Final: minor (deferred): Link::send blocking write on the UI thread
After fixes: desktop 86/86 (with COOP_TEST_SWF), core lib 129/129, SWF suite 4291 passed 0 failed, clippy only 2 upstream findings
