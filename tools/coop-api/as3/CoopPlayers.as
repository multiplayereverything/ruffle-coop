package {
    /** Coop.players: who is playing. `local` and `isHost` differ per PC: only for what this PC shows. */
    public class CoopPlayers {
        /** Number of players, 2 to 4 (1 outside a session). */
        public function count():int { return Coop.active() ? int(Coop.call("coop.players")) : 1; }

        /** This PC's slot (0 is the host). Never for gameplay. */
        public function local():int { return Coop.active() ? int(Coop.call("coop.localSlot")) : 0; }

        /** True on the host's PC. Never for gameplay. */
        public function isHost():Boolean { return Coop.active() ? Boolean(Coop.call("coop.isHost")) : true; }

        /** That player's name. */
        public function name(slot:int):String {
            var n:* = Coop.call("coop.name", slot);
            return n == undefined ? "Player " + (slot + 1) : String(n);
        }

        /** This PC's round trip to that player in ms (display only); -1 until measured. */
        public function ping(slot:int):int {
            var p:* = Coop.call("coop.ping", slot);
            return p == undefined ? -1 : int(p);
        }
    }
}
