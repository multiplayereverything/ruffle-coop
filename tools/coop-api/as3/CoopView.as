package {
    /** Coop.view: what Ruffle draws over the game (the picture only). */
    public class CoopView {
        /** The clip at path (a child of the root, "root.hud") stays put on screen whatever the camera does. */
        public function pinHud(path:String):Boolean { return Coop.call("coop.hud", path) == true; }

        /** A name tag above the clip at path; "" removes it. */
        public function label(slot:int, path:String, text:String):Boolean { return Coop.call("coop.label", slot, path, text) == true; }
    }
}
