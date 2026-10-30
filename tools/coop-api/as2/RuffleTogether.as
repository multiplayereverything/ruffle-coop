/**
 * Ruffle Together co-op API for AS2 games (any SWF version, including 6 and 7).
 * Full reference: docs/coop-api.md in the ruffle-coop repository.
 *
 * Every call is safe outside Ruffle Together (plain Flash Player or plain Ruffle): active() is false,
 * players() is 1, localSlot() is 0, mouse calls read the normal mouse, and camera, HUD, label and
 * keys calls do nothing.
 *
 * Shared answers (same on every PC, fine for gameplay): players, name, mouseX/mouseY/mouseDown(slot), has.
 * Local answers (differ per PC, never for gameplay): localSlot, isHost.
 * camera, hud and label only change the picture on each PC; keys switches the game's key map on every PC.
 */
class RuffleTogether {
    private static var checked:Boolean = false;
    private static var inSession:Boolean = false;

    /** True inside a Ruffle Together session. */
    public static function active():Boolean {
        if (!checked) {
            checked = true;
            // SWF 6 and 7 hide ExternalInterface: unhide the package and the class first
            _global.ASSetPropFlags(_global, "flash", 0, 4096);
            var ei = _global.flash.external.ExternalInterface;
            if (ei != undefined) {
                _global.ASSetPropFlags(ei, null, 0, 4096);
                inSession = ei.available && Number(ei.call("coop.players")) > 0;
            }
        }
        return inSession;
    }

    private static function send(name:String, a, b, c, d) {
        if (!active()) {
            return undefined;
        }
        var ei = _global.flash.external.ExternalInterface;
        if (d != undefined) return ei.call(name, a, b, c, d);
        if (c != undefined) return ei.call(name, a, b, c);
        if (b != undefined) return ei.call(name, a, b);
        if (a != undefined) return ei.call(name, a);
        return ei.call(name);
    }

    public static function players():Number { return active() ? Number(send("coop.players")) : 1; }
    public static function localSlot():Number { return active() ? Number(send("coop.localSlot")) : 0; }
    public static function isHost():Boolean { return active() ? Boolean(send("coop.isHost")) : true; }
    public static function version():Number { return active() ? Number(send("coop.version")) : 0; }

    /** True if this build has the feature: "camera", "hud", "label", "keys", "name", "mouse". */
    public static function has(feature:String):Boolean { return active() && send("coop.has", feature) == true; }

    public static function name(slot:Number):String {
        var n = send("coop.name", slot);
        return n == undefined ? "Player " + (slot + 1) : String(n);
    }

    public static function mouseX(slot:Number):Number { return active() ? Number(send("coop.mouseX", slot)) : _root._xmouse; }
    public static function mouseY(slot:Number):Number { return active() ? Number(send("coop.mouseY", slot)) : _root._ymouse; }
    public static function mouseDown(slot:Number):Boolean { return active() ? send("coop.mouseDown", slot) == true : false; }

    /** True on the frame that player pressed their left button. */
    public static function clicked(slot:Number):Boolean { return send("coop.clicked", slot) == true; }

    /** Who may click the game: "host" (the default), "buttons" (friends on clickable things too) or "all". */
    public static function mouse(mode:String):Boolean { return send("coop.mouse", mode) == true; }

    /** A box (stage pixels) only that player may click. */
    public static function zone(slot:Number, x:Number, y:Number, w:Number, h:Number):Boolean {
        return send("coop.zone", slot, x, y, w, h) == true;
    }

    /** Clears that player's boxes. */
    public static function zoneOff(slot:Number):Boolean { return send("coop.zone", slot) == true; }

    /** Slot's camera: stage point (x, y) at the middle of that player's screen, zoom 1 = normal. Call it for every slot. */
    public static function camera(slot:Number, x:Number, y:Number, zoom:Number):Boolean {
        return send("coop.camera", slot, x, y, zoom == undefined ? 1 : zoom) == true;
    }

    public static function cameraOff(slot:Number):Boolean { return send("coop.camera", slot) == true; }

    /** The clip at path (a child of _root, e.g. "_root.hud") is drawn without the camera. */
    public static function hud(path:String):Boolean { return send("coop.hud", path) == true; }

    /** A name tag above the clip at path; "" removes it. */
    public static function label(slot:Number, path:String, text:String):Boolean {
        return send("coop.label", slot, path, text) == true;
    }

    /** The game's own key map, in the .keys format ("slot 0: UP=38 FIRE=32\n..."). */
    public static function keys(text:String):Boolean { return send("coop.keys", text) == true; }

    /** That player's held buttons as bits (UP 1, DOWN 2, LEFT 4, RIGHT 8, FIRE 16, ALT 32, KEY1 64, ...). */
    public static function buttons(slot:Number):Number { return active() ? Number(send("coop.buttons", slot)) : 0; }

    /** True while that player holds the button ("UP", "FIRE", "KEY1", ...). */
    public static function button(slot:Number, name:String):Boolean { return send("coop.button", slot, name) == true; }

    /** What that player typed this frame ("\b" = Backspace, "\n" = Enter). */
    public static function typed(slot:Number):String {
        var t = send("coop.typed", slot);
        return t == undefined ? "" : String(t);
    }

    /** The host's setting (--coop-option NAME=VALUE), or undefined. */
    public static function option(name:String) { return send("coop.option", name); }

    /** Every host setting, as an object (empty outside a session). */
    public static function options():Object {
        var o = send("coop.options");
        return o == undefined ? {} : o;
    }

    /** This PC's round trip to that player in ms (display only); undefined until measured. */
    public static function ping(slot:Number) { return send("coop.ping", slot); }

    /** What happened since the last call: ["join 0", "leave 2", "chat 1 gg", ...]. */
    public static function events():Array {
        var e = send("coop.events");
        return e == undefined ? [] : e;
    }
}
