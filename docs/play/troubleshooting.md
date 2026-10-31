# Troubleshooting

## A friend can't join

| Cause | What to do |
|---|---|
| A different version | Everyone needs the same Ruffle Together version. Update from [Releases](https://github.com/multiplayereverything/ruffle-together/releases/latest), or let the launcher do it |
| A different game file | Everyone needs the exact same SWF. Download it again from the same place |
| A different key map | Everyone uses the same `--coop-keys` |
| The room is full or already started | Host a new room. Nobody can join a game that is running |
| Nothing happens | Check the room code. With a direct connection, check the IP and that the port is open on the host |

## The game is laggy

- The game waits for the slowest player's keys. Try a different host (the one with the best connection).
- The log, `%LOCALAPPDATA%\ruffle\log\ruffle.log`, gets a "Ruffle Together timing" line every 5 seconds: the
  frame rate, the time per frame, and how long the game waited for other players' keys.

## OUT OF SYNC

The games on the PCs differ. Every window says so, and each PC writes a report of what differs. Restart the level.
If it happens again in the same place, send the reports from every PC to the game's maker, or post them on
[Discord](https://discord.gg/2SJggtHqPR).

## The game stopped

Every PC's log says why:

| Log message | What happened |
|---|---|
| "X left (closed the game)" | That player closed their window |
| "X left (Ruffle crashed: ...)" | Ruffle crashed on that player's PC |
| "X's Ruffle is still running, but its game hasn't moved past frame N" | The game froze on that PC |
| "nothing at all from X" | That PC stopped, or its network went down |

The last 5 logs are kept as `ruffle.1.log` (the one before) to `ruffle.5.log`.
