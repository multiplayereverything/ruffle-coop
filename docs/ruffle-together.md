# Ruffle Together

Version 1.0.3 (build id `rc-1.0.3#<commit>`; players must use the same build).

Play a local-multiplayer Flash game online with up to 3 friends. Every PC runs the same game in this Ruffle
build; only the key presses travel, and every frame runs on every PC with every player's keys. Everyone gets
the same small input delay (the host picks it from the ping: 1 frame on a LAN), and the games are compared
every 25 frames: if they ever differ, every window says so ("THE TWO GAMES ARE OUT OF SYNC: restart the level").

Games with keys: Bad Ice-Cream, 2 players (`BadIceCream_coop.swf`, made by the Bad Ice-Cream mod's
`build.py --no-net`; the default), and Gun Mayhem 2, 2 to 4 players (`--coop-keys gun-mayhem-2`).
Every player needs the **same game file** and the **same Ruffle Together build**; anything else is refused
when they connect.

## Play (v1: command line)

Host, friends connect to your IP (LAN, Radmin VPN, or a forwarded port):

```
ruffle_desktop --coop-host 7801 --coop-name Liam BadIceCream_coop.swf
```

Friend:

```
ruffle_desktop --coop-join <host IP>:7801 --coop-name Sam BadIceCream_coop.swf
```

Through the central relay with a room code (no IP needed):

```
ruffle_desktop --coop-host-relay 195.60.166.57:25569 BadIceCream_coop.swf      (prints the room code)
ruffle_desktop --coop-join-relay 195.60.166.57:25569 --coop-code ABCDE BadIceCream_coop.swf
```

The relay needs the `ruffle-coop` game in mods-info first.

- Each player uses their own normal keys: arrows or WASD to move. Bad Ice-Cream: Space, Enter or F to shoot.
  Gun Mayhem 2: Z or T to shoot, X or Y for a grenade. The host is player 1, friends follow in join order.
- `--coop-players N` (host): start with N players (2-4, as many as the game has keys for); the game starts
  once all have joined. `--coop-keys <game>`: every player must give the same.
- Only the host's mouse works in the game (menus, level select).
- `--coop-delay N` (host): a fixed input delay in frames. Without it the host pings the friends when they
  connect and picks the smallest delay that runs smoothly (about 1 frame per 67 ms of ping; with 3-4
  players a friend's keys go through the host, so the two worst pings count together).
- The window title shows the room code, your player number, and "waiting for…", "… left" or "OUT OF SYNC".
- Everyone plays with the host's save (unlocked levels); nobody's own save changes.
- Every 5 seconds the log (`%LOCALAPPDATA%\ruffle\log\ruffle.log`) gets a "Ruffle Together timing" line:
  frame rate, time per frame, and how long the game waited for the other PC's keys.

## Testing (no windows, no real keyboard)

- `--coop-replay <swf> --coop-script <file> --coop-out <file>`: runs the game headless and writes its
  fingerprint every frame. The same script must give the same file every time.
- `--coop-headless` with the host/join flags and `--coop-script`: a whole session without a window; prints
  `COOP RESULT ...` (sync, input delay).
- `python tools/coop-lab/lab.py --swf <swf> [--players 4 --keys gun-mayhem-2]`: players on one PC, through
  a local relay with lag and directly.
- `python tools/coop-lab/windows_lab.py --swf <swf>`: two real windows, checks that every frame reaches the
  screen, with and without mouse-move messages (posted to the two windows only).
- `COOP_TEST_SWF=<swf> cargo test -p ruffle_desktop --release coop::` runs the session tests on the game.

## Known limits (v1)

- Keys for Bad Ice-Cream and Gun Mayhem 2 only (Gun Mayhem 2 uses its default controls: changed controls in
  the host's save would not match); P (pause), M (mute) and Esc don't reach the game in a session.
- If anyone leaves, the session ends for everyone.
- During a session there is no menu, no fullscreen, and no opening another file; the stage keeps the game's
  own size (the window can still be resized).
- Games whose logic waits for a sound to finish, reads a sound's position, or loops over a Dictionary keyed by
  objects may drift; the sync check warns if they do.
- Network loads (URLLoader, Loader from the web) don't happen in a session.
