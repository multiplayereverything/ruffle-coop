package {
    /** Coop.screens: pop-ups only one player sees and uses (a shop, an inventory, a player editor). */
    public class CoopScreens {
        /** The clip at path belongs to whoever's click opened it, while it is on show. */
        public function only(path:String):Boolean { return Coop.call("coop.only", path) == true; }

        /** The same, always that player's. */
        public function onlyFor(slot:int, path:String):Boolean { return Coop.call("coop.only", slot, path) == true; }

        /** The same, and while it is someone's nobody else may click the buttons that open it ("_root.players.*.instance*"). */
        public function onlyOpenedBy(path:String, pattern:String):Boolean { return Coop.call("coop.only", path, pattern) == true; }

        /** Everyone sees and clicks the clip at path again. */
        public function share(path:String):Boolean { return Coop.call("coop.shared", path) == true; }
    }
}
