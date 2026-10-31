# Helper library

`RuffleTogether.as` wraps every call, for
[AS2](https://github.com/multiplayereverything/ruffle-together/blob/coop/tools/coop-api/as2/RuffleTogether.as) (any
SWF version: it unhides `ExternalInterface` on SWF 6 and 7 for you) and
[AS3](https://github.com/multiplayereverything/ruffle-together/blob/coop/tools/coop-api/as3/RuffleTogether.as).
Both have the same methods.

**Outside Ruffle Together every method is safe:** `active()` is false, `players()` is 1, `localSlot()` is 0, and the
camera, HUD, label and key map calls do nothing. Mouse calls read the normal mouse in AS2 and return `NaN` in AS3
(use your own mouse). One SWF works everywhere.

| Method | Call |
|---|---|
| `active()` | `true` inside a Ruffle Together session |
| `players()`, `name(slot)`, `version()`, `has(feature)` | [Players](players.md) |
| `localSlot()`, `isHost()`, `ping(slot)` | [Players](players.md) (local) |
| `buttons(slot)`, `button(slot, name)` | [Input](input.md) |
| `mouseX(slot)`, `mouseY(slot)`, `mouseDown(slot)`, `clicked(slot)` | [Input](input.md) |
| `typed(slot)`, `keys(text)` | [Input](input.md) |
| `mouse(mode)`, `hover(mode)` | [The mouse](mouse.md) |
| `zone(slot, x, y, w, h)`, `zoneClip(slot, path)`, `zoneOff(slot)` | `coop.zone` |
| `only(path)`, `onlyFor(slot, path)`, `onlyOpenedBy(path, pattern)`, `shared(path)` | [Private screens](mouse.md#private-screens) |
| `option(name)`, `options()` | [Host settings](host-settings.md) |
| `camera(slot, x, y, zoom)`, `cameraOff(slot)`, `hud(path)`, `label(slot, path, text)` | [Camera, HUD, labels](picture.md) |
| `events()` | [Events and chat](events.md) |

## AS2

```actionscript
// SWF 6 to 8: put RuffleTogether.as on the class path
if (RuffleTogether.active()) {
    for (var slot = 0; slot < RuffleTogether.players(); slot++) {
        _root["hero" + slot].move(RuffleTogether.buttons(slot));
    }
}
```

## AS3

```actionscript
import RuffleTogether;

if (RuffleTogether.active()) {
    trace("players: " + RuffleTogether.players());
}
```
