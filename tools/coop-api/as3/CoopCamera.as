package {
    /**
     * Coop.camera: a camera per player (the picture only: the game never sees it). Set every player's
     * camera, on every PC: each PC draws with its own player's.
     */
    public class CoopCamera {
        private var xs:Array = [0, 0, 0, 0];
        private var ys:Array = [0, 0, 0, 0];
        private var zooms:Array = [1, 1, 1, 1];

        /** That player's camera looks at the stage point (x, y), at the middle of their screen. */
        public function setPosition(slot:int, x:Number, y:Number):Boolean {
            xs[slot] = x;
            ys[slot] = y;
            return send(slot);
        }

        /** 1 is normal size, 2 twice as big. */
        public function setZoom(slot:int, zoom:Number):Boolean {
            zooms[slot] = zoom;
            return send(slot);
        }

        /** Position and zoom at once. */
        public function setView(slot:int, x:Number, y:Number, zoom:Number = 1):Boolean {
            xs[slot] = x;
            ys[slot] = y;
            zooms[slot] = zoom;
            return send(slot);
        }

        /** That player's screen shows the stage as it is again. */
        public function off(slot:int):Boolean { return Coop.call("coop.camera", slot) == true; }

        private function send(slot:int):Boolean { return Coop.call("coop.camera", slot, xs[slot], ys[slot], zooms[slot]) == true; }
    }
}
