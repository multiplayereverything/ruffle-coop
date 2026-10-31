package {
    import flash.external.ExternalInterface;

    /**
     * Ruffle Together co-op API for AS3 games.
     * The older flat helper: new code uses the grouped library (Coop.as). Full reference: docs/coop-api.md.
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

        /** True on the frame that player pressed their left button. */
        public static function clicked(slot:int):Boolean { return send("coop.clicked", slot) == true; }

        /** Who may click the game: "host" (the default), "zones" (friends only in their own zone), "buttons" (friends on clickable things) or "all". */
        public static function mouse(mode:String):Boolean { return send("coop.mouse", mode) == true; }

        /** A box (stage pixels) only that player may click. */
        public static function zone(slot:int, x:Number, y:Number, w:Number, h:Number):Boolean {
            return send("coop.zone", slot, x, y, w, h) == true;
        }

        /** The buttons inside the clip at path ("_root.players.p2") belong to that player, while it is on show. */
        public static function zoneClip(slot:int, path:String):Boolean { return send("coop.zone", slot, path) == true; }

        /** A private screen: the clip at path is seen and clicked only by the player whose click opened it. */
        public static function only(path:String):Boolean { return send("coop.only", path) == true; }

        /** A private screen that is always that player's. */
        public static function onlyFor(slot:int, path:String):Boolean { return send("coop.only", slot, path) == true; }

        /** A private screen, and while it is someone's nobody else may click the buttons that open it ("_root.players.*.instance*"). */
        public static function onlyOpenedBy(path:String, pattern:String):Boolean { return send("coop.only", path, pattern) == true; }

        /** How each screen draws Button symbols while friends have mice: "own" (the default) or "shared". */
        public static function hover(mode:String):Boolean { return send("coop.hover", mode) == true; }

        /** Everyone sees and clicks the clip at path again. */
        public static function shared(path:String):Boolean { return send("coop.shared", path) == true; }

        /** Clears that player's zones. */
        public static function zoneOff(slot:int):Boolean { return send("coop.zone", slot) == true; }

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

        /** That player's held buttons as bits (UP 1, DOWN 2, LEFT 4, RIGHT 8, FIRE 16, ALT 32, KEY1 64, ...). */
        public static function buttons(slot:int):int { return active() ? int(send("coop.buttons", slot)) : 0; }

        /** True while that player holds the button ("UP", "FIRE", "KEY1", ...). */
        public static function button(slot:int, name:String):Boolean { return send("coop.button", slot, name) == true; }

        /** What that player typed this frame ("\b" = Backspace, "\n" = Enter). */
        public static function typed(slot:int):String {
            var t:* = send("coop.typed", slot);
            return t == undefined ? "" : String(t);
        }

        /** The host's setting (--coop-option NAME=VALUE), or null. */
        public static function option(name:String):String {
            var o:* = send("coop.option", name);
            return o == undefined ? null : String(o);
        }

        /** Every host setting, as an object (empty outside a session). */
        public static function options():Object {
            var o:* = send("coop.options");
            return o == undefined ? {} : o;
        }

        /** This PC's round trip to that player in ms (display only); -1 until measured. */
        public static function ping(slot:int):int {
            var p:* = send("coop.ping", slot);
            return p == undefined ? -1 : int(p);
        }

        /** What happened since the last call: ["join 0", "leave 2", "chat 1 gg", ...]. */
        public static function events():Array {
            var e:* = send("coop.events");
            return e == undefined ? [] : e as Array;
        }
    }
}
