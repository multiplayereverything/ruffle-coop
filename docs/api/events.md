# Events and chat

| Call | Kind | Answer |
|---|---|---|
| `coop.events` | shared | A list of what happened since the last call, oldest first (at most 100 kept) |

| Event | Meaning |
|---|---|
| `"join N"` | Player N is in the game (every player at the start) |
| `"leave N"` | Player N left. From this frame they hold nothing |
| `"chat N text"` | Player N sent a chat line |

Read it **once per frame**: each call returns only what happened since the last one.

```actionscript
for each (var event:String in Coop.room.events()) {
    var parts:Array = event.split(" ");
    var slot:int = int(parts[1]);
    if (parts[0] == "leave") heroes[slot].visible = false;
    if (parts[0] == "chat") showBubble(slot, parts.slice(2).join(" "));
}
```

Chat needs no code: `/` opens a chat box, and lines show in the bottom-left corner. The `chat` event is for games
that want to show chat their own way, such as speech bubbles.
