# Making a game co-op

There are three ways to get a Flash game into Ruffle Together. Pick the first one that fits.

::::{grid} 1
:gutter: 3

:::{grid-item-card} 1. The game already has local multiplayer
Two players on one keyboard, like Gun Mayhem 2 or Fireboy & Watergirl. **Write a key map**: a short text file
that says which keys each player's buttons press. The SWF doesn't change at all.

{doc}`Key maps → <key-maps>`
:::

:::{grid-item-card} 2. The game is single-player
Add players to the SWF: extra heroes that each follow one player's buttons, read through the co-op API.
Cameras, name tags and the player count come with it. Most games are someone else's SWF: decompile it and add
your code next to the game's.

{doc}`Your first co-op game → <first-game>` · {doc}`Modding a game you didn't write → <existing-game>`
:::

:::{grid-item-card} 3. You're making a new game
Build it with the [Coop library](../api/library.md) from the start. It works as a normal single-player
game outside Ruffle Together, so one SWF works everywhere.

{doc}`API reference → <../api/index>`
:::
::::

Whichever path you take, **test it without anyone playing**: replays must give the same result every time, and
two players through the relay must never go out of sync. See {doc}`testing`.

:::{seealso}
The [Flash Multiplayer Skill](https://github.com/multiplayereverything/flash-multiplayer-skill) teaches an AI agent
all of this, from reading a SWF to shipping it, with checks at every step.
:::
