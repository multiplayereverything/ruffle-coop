package {
    /** Coop.zones: clips and boxes only one player may click. */
    public class CoopZones {
        /** The buttons inside the clip at path ("_root.players.p2") are that player's, while it is on show. */
        public function give(slot:int, path:String):Boolean { return Coop.call("coop.zone", slot, path) == true; }

        /** A box (stage pixels) that is that player's on every screen. */
        public function giveBox(slot:int, x:Number, y:Number, width:Number, height:Number):Boolean {
            return Coop.call("coop.zone", slot, x, y, width, height) == true;
        }

        /** Clears that player's zones. */
        public function clear(slot:int):Boolean { return Coop.call("coop.zone", slot) == true; }
    }
}
