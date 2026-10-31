# The co-op API

A Flash game, AS2 or AS3, can ask Ruffle Together about the players and change how co-op works for it. This is
version **2** of the API.

## How to call it

Every call goes through `ExternalInterface.call(name, args...)`, the same in AS2 and AS3:

```actionscript
var players = ExternalInterface.call("coop.players");
var p2Fire = ExternalInterface.call("coop.button", 1, "FIRE");
```

Easier: the [helper library](library.md), `RuffleTogether.players()`, `RuffleTogether.camera(0, x, y, 2)` and so on.
Outside Ruffle Together it answers like a single-player game (1 player, slot 0, no camera), so one SWF works
everywhere.

- **In a session or not:** `ExternalInterface.available && ExternalInterface.call("coop.players") > 0`. In plain
  Ruffle or Flash Player this is false.
- **SWF 6 and 7 (AS1/AS2):** `ExternalInterface` is hidden. Unhide it first (the AS2 library does this):
  `ASSetPropFlags(_global, "flash", 0, 4096); ASSetPropFlags(flash.external.ExternalInterface, null, 0, 4096);`
- **Slots** are players 0 to `players - 1`. Slot 0 is the host. A slot outside that range answers `undefined`.
- In a session there is no JavaScript: any other `ExternalInterface.call` returns `undefined`.

## The one rule: shared or local

Every PC runs the same game. Each answer is one of these kinds:

| Kind | Meaning | Use it for |
|---|---|---|
| **shared** | The same answer on every PC on the same frame | Anything, gameplay included |
| **local** | Differs between PCs | Only what this PC shows: text, colours, sounds. Never gameplay, and never moving, showing or creating display objects |
| **picture only** | Changes only what each PC draws | Cameras, HUD, labels: the game never sees them |

The sync check compares every object's position, visibility, depth and frame. A local value that reaches gameplay
makes the PCs' games split, and the window title says OUT OF SYNC.

## Every call

| Call | Kind | Page |
|---|---|---|
| `coop.players`, `coop.name`, `coop.version`, `coop.has` | shared | [Players](players.md) |
| `coop.localSlot`, `coop.isHost`, `coop.ping` | **local** | [Players](players.md) |
| `coop.buttons`, `coop.button`, `coop.mouseX`, `coop.mouseY`, `coop.mouseDown`, `coop.clicked`, `coop.typed`, `coop.keys` | shared | [Input](input.md) |
| `coop.mouse`, `coop.zone`, `coop.hover`, `coop.only`, `coop.shared` | shared | [The mouse](mouse.md) |
| `coop.option`, `coop.options` | shared | [Host settings](host-settings.md) |
| `coop.camera`, `coop.hud`, `coop.label` | picture only | [Camera, HUD, labels](picture.md) |
| `coop.events` | shared | [Events and chat](events.md) |

Many features need no call at all: see [What every game gets](automatic.md).
