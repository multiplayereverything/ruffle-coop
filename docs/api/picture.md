# Camera, HUD, labels

These change only the picture each PC draws. Positions, hit tests, `BitmapData.draw` of the stage and the sync
check never see them.

| Call | Library | Effect |
|---|---|---|
| `coop.camera(slot, x, y, zoom)` | `Coop.camera.setView`, `setPosition`, `setZoom` | That player's camera: the stage point (x, y) at the middle of their screen. `zoom` 1 is normal (the default) |
| `coop.camera(slot)` | `Coop.camera.off` | Turns that player's camera off |
| `coop.hud(path)` | `Coop.view.pinHud` | The clip at `path` stays fixed on screen. It must be a direct child of the root (`"_root.hud"`, `"root.hud"`). The mark lasts as long as the clip |
| `coop.label(slot, path, text)` | `Coop.view.label` | A name tag above the clip at `path`, drawn by Ruffle over the game on every PC. `""` removes it |

Paths are `_root`, `_level0` or `root`, then instance names joined by dots.

## Cameras

Call `coop.camera` for **every** slot, each frame or whenever it changes, not just your own. Every PC then holds the
same cameras and draws with its own slot's.

```actionscript
for (var slot:int = 0; slot < Coop.players.count(); slot++) {
    Coop.camera.setView(slot, heroes[slot].x, heroes[slot].y, 1.5);
}
```

Each PC's mouse goes through its own camera, so `coop.mouseX/Y(slot)` and the host's normal mouse stay in game
coordinates.

:::{tip}
Don't move the world to follow the local player. That moves display objects on one PC only and splits the games.
Use a camera.
:::

## HUD

Mark score bars and menus with `coop.hud(path)` so they stay put with every camera. It works even in games that
scroll by moving `_root` itself.

## Labels

Ruffle draws labels over the game: white text with a shadow, centred above the clip's top edge, following each PC's
camera. The game never sees them.

```actionscript
Coop.view.label(1, "_root.hero1", Coop.players.name(1));
```
