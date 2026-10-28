# Ruffle Together

Play a local-multiplayer Flash game online with a friend. Both PCs run the same game in this Ruffle build;
only the key presses travel, and every frame runs on both PCs with both players' keys. Both players get
the same small input delay (3 frames by default), and the two games are compared every 25 frames: if they
ever differ, both windows say so ("THE TWO GAMES ARE OUT OF SYNC: restart the level").

First game: Bad Ice-Cream (`BadIceCream_coop.swf`, made by the Bad Ice-Cream mod's `build.py --no-net`).
Both players need the **same game file** and the **same Ruffle Together build**; anything else is refused
when you connect.

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

- Each player uses their own normal keys: arrows or WASD to move, Space, Enter or F to shoot. The host is
  player 1, the friend player 2.
- Only the host's mouse works in the game (menus, level select).
- `--coop-delay N` (host): input delay in frames. Raise it by one if the game keeps pausing on a slow connection.
- The window title shows the room code, your player number, and "waiting for…", "… left" or "OUT OF SYNC".
- Everyone plays with the host's save (unlocked levels); nobody's own save changes.

## Testing (no windows, no real keyboard)

- `--coop-replay <swf> --coop-script <file> --coop-out <file>`: runs the game headless and writes its
  fingerprint every frame. The same script must give the same file every time.
- `--coop-headless` with the host/join flags and `--coop-script`: a whole session without a window; prints
  `COOP RESULT ...` (sync, input delay).
- `python tools/coop-lab/lab.py --swf <swf>`: two players on one PC, through a local relay with lag and
  directly.
- `COOP_TEST_SWF=<swf> cargo test -p ruffle_desktop --release coop::` runs the session tests on the game.
