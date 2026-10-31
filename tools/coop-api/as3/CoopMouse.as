package {
    /**
     * Coop.mouse: every player's own mouse. Each friend's mouse is in the game only where the mode
     * lets it be (off by default); there it hovers and clicks like the host's.
     */
    public class CoopMouse {
        /** That player's mouse in stage pixels (NaN outside a session: use the game's own mouse). */
        public function x(slot:int):Number { return Coop.active() ? Number(Coop.call("coop.mouseX", slot)) : NaN; }
        public function y(slot:int):Number { return Coop.active() ? Number(Coop.call("coop.mouseY", slot)) : NaN; }

        /** True while that player's left button is down. */
        public function down(slot:int):Boolean { return Coop.call("coop.mouseDown", slot) == true; }

        /** True on the frame that player pressed their left button (whether or not it reached the game). */
        public function clicked(slot:int):Boolean { return Coop.call("coop.clicked", slot) == true; }

        /** The path of what that player's mouse is on ("_root.menu.sword"), or null. */
        public function hovered(slot:int):String {
            var h:* = Coop.call("coop.mouse.hovered", slot);
            return h == undefined ? null : String(h);
        }

        /** Inside a mouse handler (CLICK, MOUSE_DOWN, onRelease...), whose mouse caused it; -1 elsewhere. */
        public function clicker():int {
            var c:* = Coop.call("coop.mouse.clicker");
            return c == undefined ? -1 : int(c);
        }

        /**
         * Where friends' mice are in the game: "host" (nowhere, the default), "zones" (on clickable things in
         * their own zone), "buttons" (on clickable things) or "all". Resets setModeFor.
         */
        public function setMode(mode:String):Boolean { return Coop.call("coop.mouse", mode) == true; }

        /** The same for one friend (slot 1 to 3). */
        public function setModeFor(slot:int, mode:String):Boolean { return Coop.call("coop.mouse", slot, mode) == true; }

        /** "own": each screen draws Button symbols as its own player's mouse sees them (the default); "shared": as the game has them. */
        public function setHover(mode:String):Boolean { return Coop.call("coop.hover", mode) == true; }

        /** Friends' cursors on each screen: "auto" (where they may click, the default), "on" or "off". */
        public function setCursors(mode:String):Boolean { return Coop.call("coop.mouse.cursors", mode) == true; }
    }
}
