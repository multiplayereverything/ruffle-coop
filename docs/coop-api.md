# Ruffle Together co-op API (version 2)

Every call a Flash game (AS2 or AS3) can make to Ruffle Together, and what Ruffle Together does for a game
without being asked. The user guide is [ruffle-together.md](ruffle-together.md); the code is
`desktop/src/coop/api.rs` (answers), `frame.rs` (the Stepper), `session.rs` (the network) and
`core/src/coop_view.rs` (camera and HUD).

## How to call it

Every call goes through `ExternalInterface.call(name, args...)`, the same in AS2 and AS3:

```actionscript
var players = ExternalInterface.call("coop.players");
```

Easier: the helper library, `tools/coop-api/as2/RuffleTogether.as` (AS2, any SWF version) or
`tools/coop-api/as3/RuffleTogether.as` (AS3): `RuffleTogether.players()`, `RuffleTogether.camera(0, x, y, 2)`,
and so on. Outside Ruffle Together it answers like a single-player game (1 player, slot 0, no camera), so one
SWF works everywhere.

- **In a session or not:** `ExternalInterface.available && ExternalInterface.call("coop.players") > 0`.
- **SWF 6 and 7 (AS1/AS2):** `ExternalInterface` is hidden; unhide it first (the AS2 library does this):
  `ASSetPropFlags(_global, "flash", 0, 4096); ASSetPropFlags(flash.external.ExternalInterface, null, 0, 4096);`
- **Slots** are players 0 to `players - 1`; 0 is the host. A slot outside that range answers `undefined`.

## The one rule: shared or local

| Kind | Meaning | Use it for |
|---|---|---|
| **shared** | The same answer on every PC on the same frame | Anything, gameplay included |
| **local** | Differs between PCs | Only what this PC shows: text, colours, sounds. Never gameplay, never moving, showing or creating display objects |
| **picture only** | Changes only what each PC draws | Cameras, HUD, labels: the game never sees them |

Every PC runs the same game; the sync check compares every object's position, visibility, depth and frame. A
local value that reaches gameplay makes the PCs' games split (the window title says OUT OF SYNC).

## Calls

### Players

| Call | Kind | Answer |
|---|---|---|
| `coop.players` | shared | Number of players, 2 to 4 |
| `coop.name(slot)` | shared | That player's name (what they joined with) |
| `coop.localSlot` | **local** | This PC's slot |
| `coop.isHost` | **local** | `true` on the host's PC |
| `coop.ping(slot)` | **local** | This PC's round trip to that player in ms (0 to itself; `undefined` until measured, about 2 s). Friend to friend goes through the host, so it's both trips added |
| `coop.version` | shared | API version: 2 |
| `coop.has(feature)` | shared | `true` if this build has it: `players`, `mouse`, `name`, `camera`, `hud`, `label`, `keys`, `buttons`, `typed`, `options`, `ping`, `events`, `chat`, `clicks`, `zones` |

### Input

| Call | Kind | Answer |
|---|---|---|
| `coop.buttons(slot)` | shared | That player's held buttons as bits: UP 1, DOWN 2, LEFT 4, RIGHT 8, FIRE 16, ALT 32, KEY1 64, KEY2 128, KEY3 256, KEY4 512, VIEW 1024, QUIT 2048, YES 4096, NO 8192, MENU 16384 |
| `coop.button(slot, name)` | shared | `true` while that player holds the button (`"FIRE"`, `"KEY1"`...); `undefined` for an unknown name |
| `coop.mouseX(slot)`, `coop.mouseY(slot)` | shared | That player's mouse in stage pixels (each PC's cursor goes through its own camera, so these are game coordinates). (0, 0) until it moves |
| `coop.mouseDown(slot)` | shared | `true` while that player's left button is down |
| `coop.clicked(slot)` | shared | `true` on the frame that player pressed their left button (whether or not the click reached the game) |
| `coop.typed(slot)` | shared | What that player typed this frame, `"\b"` for Backspace and `"\n"` for Enter. Calling it tells Ruffle the game wants every typed character for the next 60 frames (see "Typing" below) |
| `coop.keys(text)` | shared | The game's own key map, in the `.keys` format (`"slot 0: UP=38 FIRE=32\n..."`), used from the next frame on every PC; held keys are let go first; the same map again changes nothing. `true` if it parsed |

`coop.buttons` is the easy way to add players to a single-player game: read each slot's buttons and move
each character, instead of faking keyboard keys.

### Who may click

Flash has one mouse. By default only the host's clicks reach the game (every PC still knows every player's
mouse through `coop.mouseX` and friends). A game can let friends click too:

| Call | Kind | Effect |
|---|---|---|
| `coop.mouse(mode)` | shared | `"host"` (the default): only the host clicks. `"zones"`: a friend's clicks count only inside their own zone (below) and on something clickable there, like their own card on a character screen; the host clicks everything else. `"buttons"`: friends' clicks too, but only on something the game made clickable (what shows the hand cursor: buttons, button-mode clips, links); a friend's click anywhere else does nothing. `"all"`: everyone's clicks, anywhere |
| `coop.zone(slot, x, y, w, h)` | shared | A box (stage pixels) only that player may click, e.g. their own card on a character screen; nobody else's click inside counts, the host's included. Areas outside every box are open. `coop.zone(slot)` clears that player's boxes |

A key map can set both without changing the SWF: `mouse: zones` and `zone 1: 290,95 200x190` (player 2's
box). Chaos Faction 2's built-in map does this for its character screen. A friend's click moves the game's mouse to their point for the click and back to the host's after, so
hover effects can flicker while friends click. One click at a time: a click while another player's button is
down is dropped. For hover or drags of its own (a slider every player can move), a game sets `coop.mouse("host")`
on that screen and uses `coop.mouseX/mouseY/mouseDown(slot)` and `coop.clicked(slot)` itself.

### Host settings

| Call | Kind | Answer |
|---|---|---|
| `coop.option(name)` | shared | The host's `--coop-option NAME=VALUE` as text, or `undefined` |
| `coop.options` | shared | Every option, as an object |

The host picks them when starting (the launcher can add them); a friend's own are ignored. The SWF decides
what each name means and its default. For a launcher dropdown, list them in the mod's mods-info entry.

### Camera, HUD, labels (picture only)

| Call | Effect |
|---|---|
| `coop.camera(slot, x, y, zoom)` | That player's camera: the stage point (x, y) at the middle of their screen, `zoom` 1 = normal (default 1). Call it for **every** slot (each frame, or when it changes): every PC holds all cameras and draws with its own slot's |
| `coop.camera(slot)` | Turns that player's camera off |
| `coop.hud(path)` | The clip at `path` stays fixed on screen. It must be a direct child of the root (`"_root.hud"`, `"root.hud"`) |
| `coop.label(slot, path, text)` | A name tag above the clip at `path`, drawn by Ruffle over the game on every PC; `""` removes it |

The camera only changes the final picture on screen: positions, hit tests, `BitmapData.draw` of the stage and
the sync check never see it. Paths are `_root` / `_level0` / `root` and then instance names joined by dots.

### Events and chat

| Call | Kind | Effect |
|---|---|---|
| `coop.events` | shared | A list of what happened since the last call, oldest first (at most 100 kept): `"join N"` (every player at the start), `"leave N"` (a friend left; from this frame they hold nothing), `"chat N text"` (a chat line). Read it once per frame |

## What every game gets without calling anything

| Feature | What happens |
|---|---|
| **Key maps** | Each player's keys press that player's keys in the game (`--coop-keys NAME` or a `.keys` file; 13 built in) |
| **Typing** | What players type goes into the game's selected text box on every PC (names, chat), in player order, Backspace and Enter included. While no text box is selected, typed text only reaches `coop.typed`; instead each player's key press types its game key's character (from any key map, `coop.keys` included, and in scripts and replays), so `on(keyPress "1")` buttons work but stray keys can't trigger a game's shortcuts. A paste becomes the sender's clipboard text (64 characters at most); copy is ignored and cut deletes |
| **Leaving** | A friend who leaves or drops out no longer stops the game: everyone else plays on and sees "Sam left". Losing the host still ends it |
| **Friends' cursors** | Off by default. While friends may click (`mouse: zones`, `buttons` or `all`), a player's mouse shows in the other windows (an arrow in their colour with their name, picture only) only while it is over something that player may click, e.g. their own card on a character screen |
| **Chat** | `/` opens a chat box (unless the key map gives `/` to the game); Enter sends, Esc closes. Keys don't reach the game while it's open. Lines show bottom left and reach the game as `chat` events |
| **Same game everywhere** | Clock, `Date`, `Math.random`, timers, sound endings, script time limits and window size are the same on every PC |
| **Save** | Everyone starts from the host's co-op save (`--coop-save coop\|solo\|new`), and every PC keeps the session's saves in its own co-op save, so anyone can host next time |
| **Sync check** | Every 25 frames; the title says OUT OF SYNC and each PC writes a report of what differs |
| **Discord** | "Ruffle Together: Playing GAME, room ABCDE (2 of 2)" on the profile of anyone who shares their activity. A Join button for relay rooms comes once the join page is set (see `desktop/src/coop/discord.rs`) |

## Join links

`ruffle-together://join/<mod id>/<room code>` (for example `ruffle-together://join/gm2/ABCDE`). The launcher
owns the link: it finds the player's copy of the game and starts Ruffle with `--coop-join-relay` and
`--coop-code`. Discord's Join button opens `JOIN_PAGE/<mod id>/<room code>`, a web page that opens the link
(Discord buttons only allow https). The launcher passes `--coop-link-id <mod id>` so a host's profile gets
the button.

## Command line (for launchers and tests)

| Flag | Meaning |
|---|---|
| `--coop-host PORT`, `--coop-host-relay RELAY` | Host directly or through the relay (prints the room code) |
| `--coop-join ADDRESS`, `--coop-join-relay RELAY --coop-code CODE` | Join |
| `--coop-players N` | Host: 2 to 4 players |
| `--coop-keys NAME_OR_FILE` | The key map (every player the same) |
| `--coop-name NAME` | This player's name |
| `--coop-option NAME=VALUE` | Host: a game setting for `coop.option` (repeat for more) |
| `--coop-link-id ID` | The game's mods-info id, for Discord's Join button |
| `--coop-delay N` | Host: a fixed input delay in frames |
| `--coop-replay SWF --coop-script FILE --coop-out FILE` | Test: run headless and write every frame's fingerprint |
| `--coop-headless`, `--coop-frames N` | Test: a whole session without a window |

## Demo

[tools/coop-api/demo](../tools/coop-api/demo/README.md) is a small AS3 game that uses every call above and shows
the answers on screen, with a key map and input scripts for replays, headless sessions and two real windows.

## Input scripts (tests and labs)

```text
p0 UP|FIRE        player 0 holds these from now on (any button name, or 0)
wait 25           25 frames with what is held now
click 275 300 1   player 1's mouse: one frame pressed at stage pixel (275, 300)
move 100 50       the host's mouse moves
type 1 Sam{Enter} player 1 types "Sam" then Enter (8 characters a frame; {Backspace} too)
```
