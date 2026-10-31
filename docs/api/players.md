# Players

| Call | Kind | Answer |
|---|---|---|
| `coop.players` | shared | The number of players, 2 to 4 |
| `coop.name(slot)` | shared | That player's name (what they joined with) |
| `coop.localSlot` | **local** | This PC's slot |
| `coop.isHost` | **local** | `true` on the host's PC |
| `coop.ping(slot)` | **local** | This PC's round trip to that player in ms: 0 to itself, `undefined` until measured (about 2 s). Friend to friend goes through the host, so it's both trips added |
| `coop.version` | shared | The API version: 2 |
| `coop.has(feature)` | shared | `true` if this build has the feature |

## Features for `coop.has`

`players`, `mouse`, `name`, `camera`, `hud`, `label`, `keys`, `buttons`, `typed`, `options`, `ping`, `events`,
`chat`, `clicks`, `zones`, `private`, `hover`

Check a feature before using it, so the game still runs on an older Ruffle Together:

```actionscript
if (RuffleTogether.has("camera")) {
    RuffleTogether.camera(slot, x, y, 1);
}
```

## Using local answers safely

`localSlot` and `isHost` are different on every PC. Use them only for what this PC shows:

```actionscript
// fine: text in a field that exists on every PC
youText.text = (slot == RuffleTogether.localSlot()) ? "YOU" : "";

// wrong: the hero is moved on one PC only, and the games split
if (slot == RuffleTogether.localSlot()) hero.x += 5;
```
