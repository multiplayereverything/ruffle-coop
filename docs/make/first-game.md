# Your first co-op game

This walks through adding a second player to a single-player AS3 game: a hero that moves with the arrows. The
same steps work in AS2 with the AS2 library.

## 1. Add the helper library

Copy `RuffleTogether.as` from
[`tools/coop-api/as3/`](https://github.com/multiplayereverything/ruffle-together/tree/coop/tools/coop-api/as3) (or
`as2/` for AS2) next to your code. Every call is safe outside Ruffle Together: it answers like a single-player game
(1 player, slot 0), so your SWF still works in Flash Player and plain Ruffle.

## 2. One hero per player

Make a hero for every player, and move each one from **that player's buttons** instead of the keyboard:

```actionscript
var heroes:Array = [];
for (var slot:int = 0; slot < RuffleTogether.players(); slot++) {
    var hero:Hero = new Hero();
    hero.x = 100 + slot * 60;
    addChild(hero);
    heroes.push(hero);
}

addEventListener(Event.ENTER_FRAME, function (e:Event):void {
    for (var slot:int = 0; slot < heroes.length; slot++) {
        var held:int = RuffleTogether.buttons(slot);
        if (held & 4) heroes[slot].x -= 5;   // LEFT
        if (held & 8) heroes[slot].x += 5;   // RIGHT
        if (held & 16) heroes[slot].jump();  // FIRE
    }
});
```

`buttons(slot)` is the same on every PC on the same frame, so every PC moves every hero the same way. Outside
Ruffle Together there's 1 player; read the keyboard for slot 0 as before.

## 3. Give the game a key map

The key map says which keys a player presses for each button. A game can set its own, so players don't need
`--coop-keys`:

```actionscript
RuffleTogether.keys(
    "slot 0: UP=38 DOWN=40 LEFT=37 RIGHT=39 FIRE=32\n" +
    "slot 1: UP=38 DOWN=40 LEFT=37 RIGHT=39 FIRE=32\n" +
    "local: UP=38,87 DOWN=40,83 LEFT=37,65 RIGHT=39,68 FIRE=32,90");
```

## 4. A camera for each player

If the level is bigger than the screen, give each player their own camera instead of moving the level. Call it for
**every** player, every frame:

```actionscript
for (var slot:int = 0; slot < heroes.length; slot++) {
    RuffleTogether.camera(slot, heroes[slot].x, heroes[slot].y, 1);
}
RuffleTogether.hud("_root.hud");   // the score bar stays on screen
```

## 5. Names over the heroes

```actionscript
for (var slot:int = 0; slot < heroes.length; slot++) {
    RuffleTogether.label(slot, "_root.hero" + slot, RuffleTogether.name(slot));
}
```

(Give each hero an instance name, `hero0`, `hero1`, so its path can be found.)

## 6. Test it

Run it with two players on one PC, then through the relay, as in {doc}`testing`. If the title ever says
OUT OF SYNC, something in the game used an answer that differs between PCs: see {doc}`tips`.

:::{important}
**Never use `localSlot` or `isHost` for gameplay.** They are different on every PC. Don't move, show, hide or
create anything with them: the games split. Use them only for what this PC shows, such as a "YOU" text over your
own hero. See [the one rule](../api/index.md#the-one-rule-shared-or-local).
:::
