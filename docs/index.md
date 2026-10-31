# Ruffle Together

**Play local-multiplayer Flash games online with up to 3 friends.**

Ruffle Together is a fork of [Ruffle](https://ruffle.rs), the Flash Player emulator. Flash games that two to four
people once played on one keyboard become online games: every player runs the same game on their own PC, and only
the key presses and the mouse travel. The games themselves don't need to change.

Current version: **{{ release }}**.
[Download](https://github.com/multiplayereverything/ruffle-together/releases/latest) ·
[Discord](https://discord.gg/2SJggtHqPR) ·
[GitHub](https://github.com/multiplayereverything/ruffle-together)

::::{grid} 1 2 2 2
:gutter: 3

:::{grid-item-card} 🎮 Play
:link: play/getting-started
:link-type: doc

Install it, host a room, send your friends the code.
:::

:::{grid-item-card} ⚙️ How it works
:link: how-it-works
:link-type: doc

Lockstep, the frame clock, and why the games stay the same on every PC.
:::

:::{grid-item-card} 🛠️ Make a game co-op
:link: make/index
:link-type: doc

Key maps for games that already have local multiplayer, and the co-op API for everything else.
:::

:::{grid-item-card} 📖 API reference
:link: api/index
:link-type: doc

Every `coop.*` call a Flash game can make, and which answers are safe for gameplay.
:::
::::

```{toctree}
:caption: Playing
:hidden:

play/getting-started
play/in-a-game
play/games
play/tips
play/troubleshooting
```

```{toctree}
:caption: How it works
:hidden:

how-it-works
```

```{toctree}
:caption: Making games co-op
:hidden:

make/index
make/key-maps
make/first-game
make/existing-game
make/tips
make/testing
```

```{toctree}
:caption: API reference
:hidden:

api/index
api/players
api/input
api/mouse
api/host-settings
api/picture
api/events
api/automatic
api/library
api/command-line
api/input-scripts
```

```{toctree}
:caption: About
:hidden:

about/limits
about/changelog
about/credits
```
