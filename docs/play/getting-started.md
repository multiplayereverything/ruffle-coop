# Getting started

## What you need

- A Windows 10 or 11 PC (64-bit). Linux and macOS can build it from source, but that isn't tested yet.
- The game's SWF file. **Every player needs the exact same file**; a different one is refused when they connect.
- The same Ruffle Together version as your friends. Different versions can't join each other.

## The easy way: the launcher

The [Multiplayer Mods Launcher](https://github.com/multiplayereverything/multiplayer-mods-launcher/releases/latest)
keeps Ruffle Together up to date, finds your copy of each game, and starts it with the right key map and settings.
Pick a game, press **Host**, and send your friends the room code. They pick the same game, press **Join** and type
the code.

## By hand

1. Download the newest zip from
   [Releases](https://github.com/multiplayereverything/ruffle-together/releases/latest) and unzip it anywhere.
2. Open a terminal in that folder.

### Host a room

The relay gives you a 5-letter room code. No IP address and no port forwarding needed.

```console
ruffle_desktop --coop-host-relay 165.232.82.11:25569 --coop-players 2 --coop-keys gun-mayhem-2 --coop-name Liam Game.swf
```

The window shows the room code. Send it to your friends.

### Join a room

```console
ruffle_desktop --coop-join-relay 165.232.82.11:25569 --coop-code ABCDE --coop-keys gun-mayhem-2 --coop-name Sam Game.swf
```

Use the same `--coop-keys` as the host. Leave it out for Bad Ice-Cream, the default.

### Without the relay

On a LAN, over a VPN, or with a forwarded port, friends can connect straight to the host's IP:

```console
ruffle_desktop --coop-host 7801 --coop-name Liam Game.swf
ruffle_desktop --coop-join 192.168.1.20:7801 --coop-name Sam Game.swf
```

## Host options

| Option | What it does |
|---|---|
| `--coop-players N` | Start once N players (2 to 4) have joined |
| `--coop-keys <name or file>` | The key map; see [Games](games.md). Everyone uses the same |
| `--coop-name NAME` | Your name, shown to the others |
| `--coop-save coop\|solo\|new` | Which save everyone starts from (see [In a game](in-a-game.md#saves)) |
| `--coop-option NAME=VALUE` | A setting for games that read one, such as friendly fire |
| `--coop-delay N` | A fixed input delay in frames instead of the automatic one |

The full list is in the [command-line reference](../api/command-line.md).
