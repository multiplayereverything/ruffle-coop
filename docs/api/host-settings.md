# Host settings

| Call | Kind | Answer |
|---|---|---|
| `coop.option(name)` | shared | The host's `--coop-option NAME=VALUE` as text, or `undefined` |
| `coop.options` | shared | Every option, as an object |

The host picks the options when starting; a friend's own are ignored, so every PC sees the host's. The SWF decides
what each name means and what its default is.

```console
ruffle_desktop --coop-host-relay 165.232.82.11:25569 --coop-option friendly-fire=on --coop-option lives=5 Game.swf
```

```actionscript
var friendlyFire:Boolean = RuffleTogether.option("friendly-fire") == "on";
var lives:int = int(RuffleTogether.option("lives") || "3");
```

For a dropdown in the Multiplayer Mods Launcher, list the options in the game's mods-info entry.
