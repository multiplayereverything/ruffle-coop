package {
    import flash.display.Shape;
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.events.KeyboardEvent;
    import flash.text.TextField;
    import flash.text.TextFormat;

    /**
     * Ruffle Together API demo: every call in docs/coop-api.md, visible on screen.
     *
     * A 2400x1800 world (bigger than the 800x600 stage) with one dot per player, moved by that
     * player's buttons. Each player's camera follows their own dot; a fixed HUD shows the rest.
     *
     *   UP DOWN LEFT RIGHT  move your dot        FIRE  ring around your dot
     *   KEY1  switch the game's key map (menu: FIRE = Enter, play: FIRE = Space)
     *   KEY3  your camera off / on
     *   typing goes to your line of the name board; Enter makes it your dot's label
     *
     * Gameplay reads only shared answers. localSlot, isHost and ping only reach the HUD's text.
     */
    public class CoopDemo extends Sprite {
        private static const W:int = 2400;
        private static const H:int = 1800;
        private static const COLORS:Array = [0xE04040, 0x3070F0, 0x30B050, 0xE0A020];
        private static const BUTTONS:Array = ["UP", "DOWN", "LEFT", "RIGHT", "FIRE", "ALT", "KEY1", "KEY2", "KEY3", "KEY4",
            "VIEW", "QUIT", "YES", "NO", "MENU"];
        private static const FEATURES:Array = ["players", "mouse", "name", "camera", "hud", "label", "keys", "buttons",
            "typed", "options", "ping", "events", "chat", "teleport"];
        private static const MENU_KEYS:String = "slot 0: FIRE=13\nslot 1: FIRE=13\nslot 2: FIRE=13\nslot 3: FIRE=13\n";
        private static const PLAY_KEYS:String = "slot 0: FIRE=32\nslot 1: FIRE=32\nslot 2: FIRE=32\nslot 3: FIRE=32\n";

        private var world:Sprite = new Sprite();
        private var hud:Sprite = new Sprite();
        private var info:TextField;
        private var board:TextField;
        private var log:TextField;

        private var n:int;
        private var frame:int = 0;
        private var speed:Number;
        private var zoom:Number;
        private var dots:Array = [];
        private var rings:Array = [];
        private var cursors:Array = [];
        private var lines:Array = [];
        private var given:Array = [];
        private var held:Array = [];
        private var cameraOn:Array = [];
        private var menuKeys:Boolean = false;
        private var keyEvents:Array = [];
        private var events:Array = [];

        public function CoopDemo() {
            n = RuffleTogether.players();
            speed = Number(RuffleTogether.option("speed") || "6");
            zoom = Number(RuffleTogether.option("zoom") || "1");

            world.name = "world";
            addChild(world);
            drawWorld();
            for (var s:int = 0; s < n; s++) {
                var dot:Sprite = new Sprite();
                dot.name = "p" + s;
                dot.graphics.lineStyle(3, 0x000000);
                dot.graphics.beginFill(COLORS[s]);
                dot.graphics.drawCircle(0, 0, 22);
                dot.graphics.endFill();
                dot.x = 400 + s * 160;
                dot.y = 300;
                world.addChild(dot);
                dots.push(dot);
                var ring:Shape = new Shape();
                ring.graphics.lineStyle(5, COLORS[s]);
                ring.graphics.drawCircle(0, 0, 44);
                ring.visible = false;
                dot.addChild(ring);
                rings.push(ring);
                var cursor:Shape = new Shape();
                cursor.graphics.lineStyle(2, COLORS[s]);
                cursor.graphics.moveTo(-12, 0);
                cursor.graphics.lineTo(12, 0);
                cursor.graphics.moveTo(0, -12);
                cursor.graphics.lineTo(0, 12);
                cursor.graphics.beginFill(COLORS[s]);
                cursor.graphics.drawCircle(0, 0, 6);
                cursor.graphics.endFill();
                world.addChild(cursor);
                cursors.push(cursor);
                lines.push("");
                given.push("");
                held.push(0);
                cameraOn.push(true);
            }

            hud.name = "hud";
            addChild(hud);
            hud.graphics.beginFill(0x000000, 0.72);
            hud.graphics.drawRect(0, 0, 800, 150);
            hud.graphics.drawRect(0, 476, 800, 124);
            hud.graphics.endFill();
            info = text(6, 2, 788, 146, 11);
            board = text(6, 478, 390, 120, 12);
            log = text(404, 478, 390, 120, 11);

            RuffleTogether.hud("root.hud");
            RuffleTogether.keys(PLAY_KEYS);
            stage.addEventListener(KeyboardEvent.KEY_DOWN, onKey);
            addEventListener(Event.ENTER_FRAME, tick);
        }

        private function text(x:Number, y:Number, w:Number, h:Number, size:int):TextField {
            var t:TextField = new TextField();
            t.x = x;
            t.y = y;
            t.width = w;
            t.height = h;
            t.multiline = true;
            t.wordWrap = true;
            t.selectable = false;
            t.defaultTextFormat = new TextFormat("_typewriter", size, 0xFFFFFF);
            hud.addChild(t);
            return t;
        }

        private function drawWorld():void {
            var g:Shape = new Shape();
            g.graphics.beginFill(0xF4F0E6);
            g.graphics.drawRect(0, 0, W, H);
            g.graphics.endFill();
            for (var x:int = 0; x <= W; x += 100) {
                g.graphics.lineStyle(x % 400 == 0 ? 3 : 1, 0xB0A890);
                g.graphics.moveTo(x, 0);
                g.graphics.lineTo(x, H);
            }
            for (var y:int = 0; y <= H; y += 100) {
                g.graphics.lineStyle(y % 400 == 0 ? 3 : 1, 0xB0A890);
                g.graphics.moveTo(0, y);
                g.graphics.lineTo(W, y);
            }
            g.graphics.lineStyle(8, 0x806040);
            g.graphics.drawRect(0, 0, W, H);
            world.addChild(g);
            for (var gx:int = 0; gx < W; gx += 400) {
                for (var gy:int = 0; gy < H; gy += 400) {
                    var t:TextField = new TextField();
                    t.x = gx + 8;
                    t.y = gy + 6;
                    t.width = 200;
                    t.height = 40;
                    t.selectable = false;
                    t.defaultTextFormat = new TextFormat("_sans", 22, 0x9A9070, true);
                    t.text = "ABCDEF".charAt(gx / 400) + (gy / 400 + 1);
                    world.addChild(t);
                }
            }
        }

        private function onKey(e:KeyboardEvent):void {
            keyEvents.unshift(e.keyCode == 13 ? "13 Enter" : e.keyCode == 32 ? "32 Space" : String(e.keyCode));
            if (keyEvents.length > 4) {
                keyEvents.pop();
            }
        }

        private function pressed(slot:int, bits:int, bit:int):Boolean {
            return (bits & bit) != 0 && (held[slot] & bit) == 0;
        }

        private function tick(e:Event):void {
            frame++;
            for each (var ev:String in RuffleTogether.events()) {
                events.unshift("f" + frame + " " + ev);
            }
            while (events.length > 7) {
                events.pop();
            }

            for (var s:int = 0; s < n; s++) {
                var bits:int = RuffleTogether.buttons(s);
                var dot:Sprite = dots[s];
                if (bits & 1) dot.y -= speed;
                if (bits & 2) dot.y += speed;
                if (bits & 4) dot.x -= speed;
                if (bits & 8) dot.x += speed;
                dot.x = Math.max(30, Math.min(W - 30, dot.x));
                dot.y = Math.max(30, Math.min(H - 30, dot.y));
                rings[s].visible = RuffleTogether.button(s, "FIRE");
                if (pressed(s, bits, 64)) {
                    menuKeys = !menuKeys;
                    RuffleTogether.keys(menuKeys ? MENU_KEYS : PLAY_KEYS);
                }
                if (pressed(s, bits, 256)) {
                    cameraOn[s] = !cameraOn[s];
                }
                held[s] = bits;

                cursors[s].x = RuffleTogether.mouseX(s);
                cursors[s].y = RuffleTogether.mouseY(s);
                cursors[s].scaleX = cursors[s].scaleY = RuffleTogether.mouseDown(s) ? 2.5 : 1;

                var typed:String = RuffleTogether.typed(s);
                for (var i:int = 0; i < typed.length; i++) {
                    var c:String = typed.charAt(i);
                    if (c == "\b") {
                        lines[s] = lines[s].substr(0, Math.max(0, lines[s].length - 1));
                    } else if (c == "\n") {
                        given[s] = lines[s];
                        lines[s] = "";
                    } else if (lines[s].length < 24) {
                        lines[s] += c;
                    }
                }
                RuffleTogether.label(s, "root.world.p" + s, RuffleTogether.name(s) + (given[s] ? " \"" + given[s] + "\"" : ""));
                if (cameraOn[s]) {
                    RuffleTogether.camera(s, dot.x, dot.y, zoom);
                } else {
                    RuffleTogether.cameraOff(s);
                }
            }
            show();
        }

        /** The HUD's text: the only place local answers (localSlot, isHost, ping) go. */
        private function show():void {
            var me:int = RuffleTogether.localSlot();
            var has:Array = [];
            for each (var f:String in FEATURES) {
                has.push(f + (RuffleTogether.has(f) ? "+" : "-"));
            }
            var opts:Array = [];
            var all:Object = RuffleTogether.options();
            for (var k:String in all) {
                opts.push(k + "=" + all[k]);
            }
            opts.sort();
            var pings:Array = [];
            var who:Array = [];
            for (var s:int = 0; s < n; s++) {
                var p:int = RuffleTogether.ping(s);
                pings.push("P" + (s + 1) + " " + (p < 0 ? "?" : p + "ms"));
                var names:Array = [];
                var bits:int = RuffleTogether.buttons(s);
                for (var b:int = 0; b < BUTTONS.length; b++) {
                    if (bits & (1 << b)) names.push(BUTTONS[b]);
                }
                who.push("P" + (s + 1) + " " + RuffleTogether.name(s) + " at " + Math.round(dots[s].x) + "," + Math.round(dots[s].y)
                    + "  buttons=" + bits + " " + names.join("|") + (RuffleTogether.button(s, "FIRE") ? " (FIRE!)" : "")
                    + "  mouse " + Math.round(RuffleTogether.mouseX(s)) + "," + Math.round(RuffleTogether.mouseY(s))
                    + (RuffleTogether.mouseDown(s) ? " DOWN" : "") + "  camera " + (cameraOn[s] ? "on" : "OFF"));
            }
            info.text = "RUFFLE TOGETHER API DEMO   frame " + frame + "   version " + RuffleTogether.version()
                + "   players " + n + "\n"
                + "THIS PC: slot " + me + " (P" + (me + 1) + ")   isHost " + RuffleTogether.isHost() + "   ping " + pings.join("  ") + "\n"
                + "has: " + has.join(" ") + "\n"
                + "options: " + (opts.length ? opts.join("  ") : "(none)") + "   (speed " + speed + ", zoom " + zoom + ")\n"
                + who.join("\n") + "\n"
                + "key map: " + (menuKeys ? "MENU (FIRE=Enter)" : "PLAY (FIRE=Space)") + "   game got keys: " + keyEvents.join(", ");
            var text2:String = "NAME BOARD (coop.typed; Enter = label)\n";
            for (var t:int = 0; t < n; t++) {
                text2 += "P" + (t + 1) + ": " + lines[t] + "_" + (given[t] ? "   label: " + given[t] : "") + "\n";
            }
            board.text = text2;
            log.text = "EVENTS (coop.events, newest first)\n" + events.join("\n");
        }
    }
}
