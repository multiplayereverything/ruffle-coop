// Ruffle Together co-op API for AS2 games (any SWF version, 6 and up), grouped like the AS3 one:
//
//     #include "Coop.as"
//     Coop.players.count(), Coop.players.name(1)
//     Coop.camera.setPosition(1, 400, 300)
//     if (Coop.mouse.clicker() == 2) ...
//
// Full reference: docs/coop-api.md in the ruffle-coop repository.
// Every call is safe outside Ruffle Together: Coop.active() is false, one player, slot 0, mouse calls
// read the game's own mouse, and calls that change something do nothing.

if (_global.Coop == undefined) {
    _global.Coop = new Object();

    Coop.active = function () {
        if (Coop.checked == undefined) {
            Coop.checked = true;
            Coop.inSession = false;
            // SWF 6 and 7 hide ExternalInterface: unhide the package and the class first
            _global.ASSetPropFlags(_global, "flash", 0, 4096);
            var ei = _global.flash.external.ExternalInterface;
            if (ei != undefined) {
                _global.ASSetPropFlags(ei, null, 0, 4096);
                Coop.inSession = ei.available && Number(ei.call("coop.players")) > 0;
            }
        }
        return Coop.inSession;
    };

    // A raw API call by its name ("coop.camera", ...); undefined outside a session.
    Coop.call = function (name) {
        if (!Coop.active()) {
            return undefined;
        }
        var ei = _global.flash.external.ExternalInterface;
        return ei.call.apply(ei, arguments);
    };

    // Who is playing. local() and isHost() differ per PC: only for what this PC shows.
    Coop.players = {
        count: function () { return Coop.active() ? Number(Coop.call("coop.players")) : 1; },
        local: function () { return Coop.active() ? Number(Coop.call("coop.localSlot")) : 0; },
        isHost: function () { return Coop.active() ? Boolean(Coop.call("coop.isHost")) : true; },
        name: function (slot) {
            var n = Coop.call("coop.name", slot);
            return n == undefined ? "Player " + (slot + 1) : String(n);
        },
        // this PC's round trip to that player in ms (display only); -1 until measured
        ping: function (slot) {
            var p = Coop.call("coop.ping", slot);
            return p == undefined ? -1 : Number(p);
        }
    };

    // Every player's buttons and typing, the same on every PC.
    Coop.input = {
        // held buttons as bits: UP 1, DOWN 2, LEFT 4, RIGHT 8, FIRE 16, ALT 32, KEY1 64, ...
        buttons: function (slot) { return Coop.active() ? Number(Coop.call("coop.buttons", slot)) : 0; },
        held: function (slot, button) { return Coop.call("coop.button", slot, button) == true; },
        typed: function (slot) {
            var t = Coop.call("coop.typed", slot);
            return t == undefined ? "" : String(t);
        },
        setKeys: function (text) { return Coop.call("coop.keys", text) == true; }
    };

    // Every player's own mouse: a friend's is in the game only where the mode lets it be.
    Coop.mouse = {
        x: function (slot) { return Coop.active() ? Number(Coop.call("coop.mouseX", slot)) : _root._xmouse; },
        y: function (slot) { return Coop.active() ? Number(Coop.call("coop.mouseY", slot)) : _root._ymouse; },
        down: function (slot) { return Coop.call("coop.mouseDown", slot) == true; },
        clicked: function (slot) { return Coop.call("coop.clicked", slot) == true; },
        // the path of what that player's mouse is on, or null
        hovered: function (slot) {
            var h = Coop.call("coop.mouse.hovered", slot);
            return h == undefined ? null : String(h);
        },
        // inside onPress, onRelease and the like: whose mouse caused it; -1 elsewhere
        clicker: function () {
            var c = Coop.call("coop.mouse.clicker");
            return c == undefined ? -1 : Number(c);
        },
        // "host" (friends' mice off, the default), "zones", "buttons" or "all"; resets setModeFor
        setMode: function (mode) { return Coop.call("coop.mouse", mode) == true; },
        setModeFor: function (slot, mode) { return Coop.call("coop.mouse", slot, mode) == true; },
        // "own" (each screen by its own mouse, the default) or "shared"
        setHover: function (mode) { return Coop.call("coop.hover", mode) == true; },
        // friends' cursors: "auto", "on" or "off"
        setCursors: function (mode) { return Coop.call("coop.mouse.cursors", mode) == true; }
    };

    // Clips and boxes only one player may click.
    Coop.zones = {
        give: function (slot, path) { return Coop.call("coop.zone", slot, path) == true; },
        giveBox: function (slot, x, y, width, height) { return Coop.call("coop.zone", slot, x, y, width, height) == true; },
        clear: function (slot) { return Coop.call("coop.zone", slot) == true; }
    };

    // Pop-ups only one player sees and uses.
    Coop.screens = {
        only: function (path) { return Coop.call("coop.only", path) == true; },
        onlyFor: function (slot, path) { return Coop.call("coop.only", slot, path) == true; },
        onlyOpenedBy: function (path, pattern) { return Coop.call("coop.only", path, pattern) == true; },
        share: function (path) { return Coop.call("coop.shared", path) == true; }
    };

    // A camera per player (the picture only). Set every player's, on every PC.
    Coop.camera = {
        xs: [0, 0, 0, 0],
        ys: [0, 0, 0, 0],
        zooms: [1, 1, 1, 1],
        send: function (slot) { return Coop.call("coop.camera", slot, this.xs[slot], this.ys[slot], this.zooms[slot]) == true; },
        setPosition: function (slot, x, y) {
            this.xs[slot] = x;
            this.ys[slot] = y;
            return this.send(slot);
        },
        setZoom: function (slot, zoom) {
            this.zooms[slot] = zoom;
            return this.send(slot);
        },
        setView: function (slot, x, y, zoom) {
            this.xs[slot] = x;
            this.ys[slot] = y;
            this.zooms[slot] = zoom == undefined ? 1 : zoom;
            return this.send(slot);
        },
        off: function (slot) { return Coop.call("coop.camera", slot) == true; }
    };

    // What Ruffle draws over the game (the picture only).
    Coop.view = {
        pinHud: function (path) { return Coop.call("coop.hud", path) == true; },
        label: function (slot, path, text) { return Coop.call("coop.label", slot, path, text) == true; }
    };

    // The session: the host's settings, what happened, what this build can do.
    Coop.room = {
        version: function () { return Coop.active() ? Number(Coop.call("coop.version")) : 0; },
        has: function (feature) { return Coop.call("coop.has", feature) == true; },
        option: function (name) {
            var o = Coop.call("coop.option", name);
            return o == undefined ? null : String(o);
        },
        options: function () {
            var o = Coop.call("coop.options");
            return o == undefined ? {} : o;
        },
        events: function () {
            var e = Coop.call("coop.events");
            return e == undefined ? [] : e;
        }
    };
}
