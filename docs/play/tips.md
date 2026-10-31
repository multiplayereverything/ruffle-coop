# Tips for players

:::{tip}
Use the launcher. It picks the right SWF, key map and Ruffle Together version for every game, so the most common
problems (a different file, a different version) can't happen.
:::

- **The input delay depends on the worst ping.** With 3 or 4 players, a friend's keys go through the host, so the
  two worst pings count together. The player with the best connection should host.
- **A wired connection beats Wi-Fi.** The game runs at the speed of the slowest PC and the slowest connection.
- **Everyone joins before the game starts.** You can't join a game that is already running.
- **One Ruffle window per PC.** Opening another game closes the one still open.
- **Out of sync? Restart the level.** If the title says OUT OF SYNC, the games on the PCs differ. Nothing more is
  saved after that. Restarting the level, or the game, puts everyone back together.
- **Starting the co-op game over:** delete `%LOCALAPPDATA%\ruffle\SharedObjects\coop\<game file>.save` on the host,
  or host with `--coop-save new`.
- **Keys that don't do anything:** only the keys in the game's key map reach the game. `/` opens the chat unless the
  key map gives it to the game.
- **Chatting doesn't stop the game.** It keeps running for everyone while you type.
