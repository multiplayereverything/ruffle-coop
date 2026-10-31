# What every game gets

Without calling anything, every game gets these.

| Feature | What happens |
|---|---|
| **Key maps** | Each player's keys press that player's keys in the game (`--coop-keys NAME` or a `.keys` file; 13 built in) |
| **Typing** | What players type goes into the game's selected text box on every PC (names, chat), in player order, Backspace and Enter included. While no text box is selected, typed text only reaches `coop.typed`; instead each player's key press types its game key's character (from any key map, `coop.keys` included, and in scripts and replays), so `on(keyPress "1")` buttons work but stray keys can't trigger a game's shortcuts. A paste becomes the sender's clipboard text (64 characters at most); copy is ignored and cut deletes |
| **Leaving** | A friend who leaves or drops out doesn't stop the game: everyone else plays on and sees "Sam left". Losing the host still ends it |
| **Private screens** | With `private: <clip>` in the key map, a pop-up only the player who opened it sees and clicks |
| **Friends' cursors** | While friends may click (`mouse: zones`, `buttons` or `all`), a player's mouse shows in the other windows (an arrow in their colour with their name, picture only), only while it is over something that player may click |
| **Chat** | `/` opens a chat box (unless the key map gives `/` to the game); Enter sends, Esc closes. Keys don't reach the game while it's open. Lines show bottom left and reach the game as `chat` events |
| **Same game everywhere** | The clock, `Date`, `Math.random`, timers, sound endings, script time limits and the window size are the same on every PC |
| **Saves** | Everyone starts from the host's co-op save (`--coop-save coop\|solo\|new`), and every PC keeps the session's saves in its own co-op save, so anyone can host next time |
| **Sync check** | Every 25 frames. The title says OUT OF SYNC and each PC writes a report of what differs |
| **Discord** | "Ruffle Together: Playing GAME, room ABCDE (2 of 2)" on the profile of anyone who shares their activity |

## Join links

`ruffle-together://join/<mod id>/<room code>`, for example `ruffle-together://join/gm2/ABCDE`. The Multiplayer Mods
Launcher owns the link: it finds the player's copy of the game and starts Ruffle Together with `--coop-join-relay`
and `--coop-code`. The launcher passes `--coop-link-id <mod id>` so a host's Discord profile can offer a Join
button.
