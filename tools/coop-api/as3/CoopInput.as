package {
    /** Coop.input: every player's buttons and typing, the same on every PC. */
    public class CoopInput {
        /** That player's held buttons as bits: UP 1, DOWN 2, LEFT 4, RIGHT 8, FIRE 16, ALT 32, KEY1 64, ... */
        public function buttons(slot:int):int { return Coop.active() ? int(Coop.call("coop.buttons", slot)) : 0; }

        /** True while that player holds the button ("UP", "FIRE", "KEY1", ...). */
        public function held(slot:int, button:String):Boolean { return Coop.call("coop.button", slot, button) == true; }

        /** What that player typed this frame ("\b" is Backspace, "\n" Enter). */
        public function typed(slot:int):String {
            var t:* = Coop.call("coop.typed", slot);
            return t == undefined ? "" : String(t);
        }

        /** The game's own key map, in the .keys format ("slot 0: UP=38 FIRE=32\n..."), from the next frame. */
        public function setKeys(text:String):Boolean { return Coop.call("coop.keys", text) == true; }
    }
}
