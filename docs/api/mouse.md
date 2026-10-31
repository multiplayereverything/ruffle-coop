# The mouse

Flash has one mouse. By default only the host's clicks reach the game, though every PC still knows every player's
mouse through [`coop.mouseX`](input.md). A game, or its [key map](../make/key-maps.md#the-mouse), can let friends
click too.

## Who may click

| Call | Kind | Effect |
|---|---|---|
| `coop.mouse(mode)` | shared | Who clicks (below) |
| `coop.zone(slot, path)` | shared | The buttons inside the clip at `path` (`"_root.players.p2"`) belong to that player, such as their own card on a character screen. Nobody else's click on them counts, the host's included. Only while that clip is on show, so other screens are untouched. When zones nest, the most exact clip wins |
| `coop.zone(slot, x, y, w, h)` | shared | The same for a box in stage pixels. A box applies on every screen, so prefer a clip |
| `coop.zone(slot)` | shared | Clears that player's zones |
| `coop.hover(mode)` | shared | How hover looks while friends may click (below) |

### `coop.mouse` modes

`"host"` (the default)
: Only the host clicks.

`"zones"`
: A friend's clicks count only inside their own zone, and only on something clickable there. The host clicks
  everything else.

`"buttons"`
: Friends click too, but only on something the game made clickable: what shows the hand cursor (buttons,
  button-mode clips, links). A friend's click anywhere else does nothing.

`"all"`
: Everyone's clicks, anywhere.

### `coop.hover` modes

`"host"` (the default)
: The game's own hover follows the host's mouse on every screen, exactly as the game does it. Hover made by the
  game's code can only ever be one for everyone.

`"own"`
: The same, plus each PC draws Button symbols in their own Up/Over art by its own player's mouse (only Button
  symbols that have Over art; nothing custom is drawn).

`"off"`
: The game's mouse rests off the stage between clicks, so there is no hover at all (mouse-move effects stop too).

Each player's own cursor shows on their own screen either way.

### How clicks take turns

A friend's click moves the game's mouse to their point for the click and back to the host's after, so hover effects
can flicker while friends click. Flash has one mouse button, so clicks take turns:

- The host's click always goes at once (a friend's held click is let go first).
- Friends' clicks that land together wait in a queue and follow one a frame, oldest first, then by slot. With 4
  players, three friends clicking in the same frame all reach the game.
- A friend holding the button over a second while others wait is let go.

For hover or drags of its own (a slider every player can move), a game sets `coop.mouse("host")` on that screen and
uses `coop.mouseX`, `coop.mouseY`, `coop.mouseDown` and `coop.clicked` itself.

## Private screens

A pop-up that only one player sees and uses: a player editor, a shop, an inventory. Every PC runs the same game, so
the pop-up is open in everyone's game. What changes is the picture and the mouse, the same way on every PC, so it
can't desync.

| Call | Kind | Effect |
|---|---|---|
| `coop.only(path)` | shared | The clip at `path` (`"_root.pcb"`) belongs to whoever's click opened it, while it is on show: only their PC draws it, only they click it, and everyone else's mouse goes through it to what is underneath |
| `coop.only(path, pattern)` | shared | The same, and while it is someone's, nobody else may click the buttons that open it (`"_root.players.*.instance*"`: `*` is any one name, `instance*` any name starting so). A refused player sees "Bob has that open: wait until they close it" |
| `coop.only(slot, path)` | shared | The same, always that player's |
| `coop.shared(path)` | shared | Everyone sees and clicks it again |

The player who opened the pop-up clicks it even outside their zone.

:::{note}
The pop-up is still open in everyone's game. A game that freezes the screen behind its pop-ups freezes it for
everyone, and `BitmapData.draw` of the stage still sees the pop-up on every PC.
:::

## Example: a character screen

Chaos Faction 2's built-in key map gives player 2 their own card (except its Keyboard 2 drop-down, which stays the
host's), and the Player Editor to whoever opens it:

```text
mouse: zones
zone 1: _root.players.p2
zone 0: _root.players.p2.combo
private: _root.pcb opened-by _root.players.*.instance*
hover: host
```

The same from the game's code:

```actionscript
RuffleTogether.mouse("zones");
RuffleTogether.zoneClip(1, "_root.players.p2");
RuffleTogether.zoneClip(0, "_root.players.p2.combo");
RuffleTogether.onlyOpenedBy("_root.pcb", "_root.players.*.instance*");
RuffleTogether.hover("host");
```
