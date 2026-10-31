# Multiple mice: plan

Goal: every player has a real mouse in the game, and each screen shows the game's own hover art for its own
player. Status: built 2026-10-31 (see coop-log.md); per-screen hover made by game code is not part of it.

## Today

Ruffle's core has one mouse: one position (`Player::mouse_position`), one hovered object and one pressed object
(`MouseData`), one button state (`InputManager`). Friends' clicks borrow it: `Stepper::feed_mice` moves it to the
friend's point, presses, releases, and moves it back. So:

- only the host's mouse hovers (a friend's hover never reaches the game);
- every borrowed click makes the host's hover roll out and back in (flicker);
- clicks take turns in a queue, and a long hold is cut off;
- `hover: own` draws copies of AS2 button art (shapes and text only), not the buttons themselves.

## Step 1: one pointer per player in the core

New `core/src/coop_mice.rs`. A pointer is what `MouseData` and `InputManager` hold for one mouse today:
position, in-stage, hovered, pressed, left button down.

- The session gives the core every player's pointer each frame (the shared inputs, so every PC does the same).
- The core runs each pointer in slot order through the existing mouse code (`handle_event`, `update_mouse_state`):
  it swaps that pointer's state in, runs it, and swaps it out. Rollover, press, release, drag-out and the AS3
  mouse events all come from Ruffle's own code, unchanged.
- Hover is counted per object: an object rolls over when the first pointer enters it and rolls out when the last
  one leaves, so two players on one button don't fire its events twice.
- `_xmouse` / `mouseX` inside a handler is the pointer that caused the event; anywhere else it's the host's.
- `startDrag` follows the pointer that started it.
- The click rules stay as they are (`mouse:`, zones, private screens): a pointer is in the game only where its
  player may click; elsewhere it is outside the stage, so it neither hovers nor clicks.
- With `mouse: host` (the default) friends have no pointer in the game, so every game that doesn't turn friends'
  clicks on plays exactly as now (the BIC, GM2 and TW fingerprints stay the same).

This replaces the borrowed mouse in `frame.rs`: the queue, the hold limit, parking and the hover flicker go away.
Clicks no longer take turns; two players can hold buttons at the same time.

## Step 2: each screen draws its own player's button states

Picture only: the game's buttons keep their shared state; each PC draws Button symbols as its own pointer sees them.

- AS3 `SimpleButton`: it already holds its Up, Over and Down objects; this PC draws the one for its own pointer.
- AS2 buttons: Ruffle builds only the current state's children, so this PC builds the other states' art once
  (shapes, text, bitmaps, and movie clips on their first frame, with no scripts run).

## Key map and API changes

| Now | After |
|---|---|
| `hover: host` (default), `own`, `off` | `hover: own` (default: Button symbols drawn per screen) or `shared` (every screen shows every player's hover) |
| Friends' clicks queue, one per frame | Every pointer clicks at once |
| `coop.mouseX(slot)` etc. | Unchanged |
| `coop.mouse(mode)` for every friend | Also `coop.mouse(slot, mode)` and `mouse N: mode`: one player's mouse on or off (`host` means off for a friend) |
| Friends' cursors only where they may click | `coop.cursors(mode)` / `cursors: auto \| on \| off`: where they may click (default), always, or never |

## Also in this round

| Addition | Kind | What it does |
|---|---|---|
| `coop.clicker()` | shared | Inside a mouse handler (`onRelease`, `CLICK`...), the slot of the player whose mouse caused it; `undefined` elsewhere |
| `coop.hovered(slot)` | shared | The path of what that player's mouse is on (`"_root.menu.sword"`), or `undefined` |
| Idle cursors fade | picture only | A friend's cursor fades out after 3 s without moving and comes back when it moves |
| Mouse debug overlay | picture only | A toggle (`--coop-mouse-debug`, or a key) that shows every player's mouse, what it's on, and whether they may click there |

Hover made by game code (`onRollOver` that moves a clip, CF2's arrows and glow) becomes real for every player but
shows on every screen, since it changes the shared game. Making that per screen is the "try it and undo it" idea,
left out for now.

## Game-side library

For people who write their own co-op code: `tools/coop-api/as2` and `as3/RuffleTogether.as` become a grouped
library, so a game writes `Coop.camera.setPosition(1, 400, 300)` instead of `ExternalInterface.call(...)`.

| Group | Calls |
|---|---|
| `Coop.players` | `count`, `name(slot)`, `local`, `isHost`, `ping(slot)` |
| `Coop.input` | `buttons(slot)`, `held(slot, name)`, `typed(slot)`, `setKeys(text)` |
| `Coop.mouse` | `x(slot)`, `y(slot)`, `down(slot)`, `clicked(slot)`, `hovered(slot)`, `clicker()`, `setMode(mode)`, `setMode(slot, mode)`, `setHover(mode)`, `setCursors(mode)` |
| `Coop.zones` | `give(slot, path)`, `giveBox(slot, x, y, w, h)`, `clear(slot)` |
| `Coop.screens` | `private(path)`, `privateFor(slot, path)`, `openedBy(path, pattern)`, `shared(path)` |
| `Coop.camera` | `setPosition(slot, x, y)`, `setZoom(slot, zoom)`, `off(slot)` |
| `Coop.view` | `pinHud(path)`, `label(slot, path, text)` |
| `Coop.room` | `option(name)`, `options()`, `events()`, `version`, `has(feature)` |

- The library is the documented way to use the API; `docs/coop-api.md` leads with it and lists the raw
  `ExternalInterface` names underneath for anyone not using it.
- Ruffle's existing names (`"coop.camera"`, `"coop.players"`...) stay as they are, so released mods keep
  working. New calls get grouped names from the start (`"coop.mouse.hovered"`).
- Every new API call gets a library method in both AS2 and AS3 and a line in the API demo.

## Code organisation

- `core/src/coop_mice.rs`: pointers and the per-pointer run (step 1); `coop_view.rs` keeps camera, HUD, paths.
- `desktop/src/coop/mice.rs`: click rules (`may_click`, zones, private screens, the busy notice) moved out of
  `frame.rs`, which goes back to building one frame from inputs.
- Comments in the co-op files cut to one line where possible (antislop-code).
- `docs/coop-api.md`: "Who may click" and "Private screens" rewritten around pointers.

## Proof

- Unit tests: two pointers hover the same button (one rollover, one rollout), simultaneous presses on two
  buttons, a drag by player 2, a pointer refused outside its zone, one friend's mouse turned off,
  `coop.clicker` inside a handler, `coop.hovered` per player.
- Fingerprints: BIC `fa623659`, GM2 `8e30f66c`, TW `ad6fe22d` unchanged. The CF2 and demo replays change (friends
  now have pointers); each must still match itself twice.
- API demo: replay twice, 4 headless players through the relay, 4 real windows.
- CF2 in two windows: player 2's arrows light up under player 2's mouse.
