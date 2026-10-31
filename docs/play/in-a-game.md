# In a game

## Waiting for everyone

While players join, every window shows the room code (or the port), who has joined and how many the game needs,
or why it can't start. Friends who join early wait as long as the last one takes. The game starts on every PC at
the same moment.

During the game the window title shows the room code, your player number, and "waiting for…", "… left" or
"OUT OF SYNC" when something is wrong.

## Players and keys

- The host is player 1; friends follow in the order they joined.
- Each player uses their own normal keys (for most games, the arrows or WASD to move). The key map turns them
  into that player's keys in the game, so nobody has to use "player 2's keys".
- In games' own menus, the host's mouse clicks by default. Some games let friends click too, for example their
  own card on a character screen.

## Typing and chat

- **Typing:** names and chat typed into the game's own text boxes reach the game on every PC: any key, capitals,
  Backspace, Enter, paste.
- **Chat:** press `/`, type, press Enter. Esc closes the box. Your keys don't reach the game while it's open.
  Chat lines show in the bottom-left corner.

## Someone leaves

A friend who leaves or drops out doesn't end the game: everyone else plays on and sees "Sam left". If the
**host** leaves, the game ends for everyone.

When a game stops, every PC's log says why: the player closed the game, Ruffle crashed, the game froze, or the
network went quiet.

## Saves

- Everyone starts from the **host's co-op save** for that game (the host's solo save the first time), so unlocked
  levels carry over.
- Whenever the game saves, every PC writes the session's saves to its own co-op save, so **anyone can host next
  time** and carry on.
- Solo saves are never changed.
- The co-op save is `%LOCALAPPDATA%\ruffle\SharedObjects\coop\<game file>.save`. Delete it to start the co-op game
  over.
- The host can pick another start with `--coop-save`:

  | Value | Everyone starts from |
  |---|---|
  | `coop` (the default) | The host's co-op save |
  | `solo` | The host's solo save, even if they have a co-op one |
  | `new` | Nothing: the game as if new |

  With `solo` or `new`, each player's co-op save is replaced the first time the game saves.
- Once the games are out of sync, nothing more is saved.

## Discord

If you share your activity on Discord, your profile shows "Ruffle Together: Playing GAME, room ABCDE (2 of 2)".
