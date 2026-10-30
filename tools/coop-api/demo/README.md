# Co-op API demo

A small AS3 SWF that shows every call in [docs/coop-api.md](../../../docs/coop-api.md) on screen, for testing a
build in real windows. Run it after any change to the co-op API or the session, and add new calls to it.

| File | What it is |
|---|---|
| `CoopDemo.as`, `CoopDemo.swf` | The demo (uses `../as3/RuffleTogether.as`) |
| `demo.keys` | Its starting key map (arrows move, Space fires, 1 key maps, 3 camera) |
| `demo-2p.txt` | A 2-player input script, about 50 s |
| `demo-2p-windows.txt` | The same three times after a 5 s wait, for watching real windows |

## What it shows

- A 2400x1800 world (bigger than the stage) with one dot per player, moved by `coop.buttons`; a ring while
  `coop.button(slot, "FIRE")`.
- Each player's camera follows their own dot (`coop.camera`); KEY3 turns it off and on (`coop.camera(slot)`).
- Ruffle draws `coop.name` labels above the dots, plus the name typed on the board.
- A fixed HUD (`coop.hud`) with the frame, `version`, `players`, `has` (the made-up "teleport" says no),
  `localSlot`, `isHost`, `ping`, `options`, each player's buttons, mouse and camera, and the key map.
- Each player's mouse as a cursor (`mouseX`, `mouseY`, `mouseDown`).
- A name board from `coop.typed` (Backspace works, Enter makes it the label).
- KEY1 switches the game's key map with `coop.keys` (menu: FIRE = Enter, play: FIRE = Space); the HUD lists
  the key codes the game got.
- Red, Blue and Green buttons anyone may click (`coop.mouse("buttons")`) and Bob's switch only player 2 may
  click (`coop.zone`); the HUD shows each player's last click (`coop.clicked`), each window the other's cursor.
- The events log (`coop.events`) shows join, chat and leave.

Gameplay reads only shared answers; `localSlot`, `isHost` and `ping` only reach the HUD's text.

## Build

```bash
java -cp ../../asc/asc.jar macromedia.asc.embedding.Main -AS3 -strict \
  -import ../../../target/release/build/ruffle_core-*/out/playerglobal_import.abc \
  -in ../as3/RuffleTogether.as -swf CoopDemo,800,600,30 CoopDemo.as
```

(Any `playerglobal_import.abc` from a release build of `ruffle_core` works; delete the `.abc` it leaves.)

## Test

1. Replay twice; the two files must be identical:
   `ruffle_desktop --coop-replay CoopDemo.swf --coop-keys demo.keys --coop-script demo-2p.txt --coop-frames 1500 --coop-out a.txt`
2. Two headless players through the relay, both must say `desync_at=none`: host with
   `--coop-headless --coop-frames 1500 --coop-script demo-2p.txt --coop-keys demo.keys --coop-players 2
   --coop-host-relay 165.232.82.11:25569 --coop-option speed=6`, friend with the same and
   `--coop-join-relay 165.232.82.11:25569 --coop-code <code>`.
3. Two real windows the same way without `--coop-headless`, with `demo-2p-windows.txt` and a different
   `--cache-directory` each. Never drive the real keyboard or mouse.

Headless replays have no system fonts, so their screenshots (`--coop-shot`) show no text; windows do. Chat can't
be scripted: type `/` in a window to see a `chat` event.
