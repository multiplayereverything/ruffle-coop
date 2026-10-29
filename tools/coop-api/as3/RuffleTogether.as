package {
    import flash.external.ExternalInterface;

    /**
     * Ruffle Together co-op API for AS3 games.
     *
     * Every call is safe outside Ruffle Together (plain Flash Player or plain Ruffle): active() is false,
     * players() is 1, localSlot() is 0, mouse calls return NaN (use your own mouse), and camera, HUD,
     * label and keys calls do nothing.
     *
     * Shared answers (same on every PC, fine for gameplay): players, name, mouseX/mouseY/mouseDown(slot), has.
     * Local answers (differ per PC, never for gameplay): localSlot, isHost.
     * camera, hud and label only change the picture on each PC; keys switches the game's key map on every PC.
     */
    public class RuffleTogether {
        private static var checked:Boolean = false;
        private static var inSession:Boolean = false;

        /** True inside a Ruffle Together session. */
        public static function active():Boolean {
            if (!checked) {
                checked = true;
                try {
                    inSession = ExternalInterface.available && Number(ExternalInterface.call("coop.players")) > 0;
                } catch (e:Error) {
                    inSession = false;
                }
            }
            return inSession;
        }

        private static function send(name:String, ...args):* {
            if (!active()) {
                return undefined;
            }
            return ExternalInterface.call.apply(null, [name].concat(args));
        }

        public static function players():int { return active() ? int(send("coop.players")) : 1; }
        public static function localSlot():int { return active() ? int(send("coop.localSlot")) : 0; }
        public static function isHost():Boolean { return active() ? Boolean(send("coop.isHost")) : true; }
        public static function version():int { return active() ? int(send("coop.version")) : 0; }

        /** True if this build has the feature: "camera", "hud", "label", "keys", "name", "mouse". */
        public static function has(feature:String):Boolean { return active() && send("coop.has", feature) == true; }

        public static function name(slot:int):String {
            var n:* = send("coop.name", slot);
            return n == undefined ? "Player " + (slot + 1) : String(n);
        }

        public static function mouseX(slot:int):Number { return active() ? Number(send("coop.mouseX", slot)) : NaN; }
        public static function mouseY(slot:int):Number { return active() ? Number(send("coop.mouseY", slot)) : NaN; }
        public static function mouseDown(slot:int):Boolean { return active() && send("coop.mouseDown", slot) == true; }

        /** Slot's camera: stage point (x, y) at the middle of that player's screen, zoom 1 = normal. Call it for every slot. */
        public static function camera(slot:int, x:Number, y:Number, zoom:Number = 1):Boolean {
            return send("coop.camera", slot, x, y, zoom) == true;
        }

        public static function cameraOff(slot:int):Boolean { return send("coop.camera", slot) == true; }

        /** The clip at path (a child of the root, e.g. "root.hud") is drawn without the camera. */
        public static function hud(path:String):Boolean { return send("coop.hud", path) == true; }

        /** A name tag above the clip at path; "" removes it. */
        public static function label(slot:int, path:String, text:String):Boolean {
            return send("coop.label", slot, path, text) == true;
        }

        /** The game's own key map, in the .keys format ("slot 0: UP=38 FIRE=32\n..."). */
        public static function keys(text:String):Boolean { return send("coop.keys", text) == true; }
    }
}
