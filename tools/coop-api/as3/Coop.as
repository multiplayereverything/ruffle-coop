package {
    import flash.external.ExternalInterface;

    /**
     * Ruffle Together co-op API for AS3 games, grouped:
     *
     *     Coop.players.count(), Coop.players.name(1)
     *     Coop.camera.setPosition(1, 400, 300)
     *     if (Coop.mouse.clicker() == 2) ...
     *
     * Copy every Coop*.as file of this folder into the game's source folder.
     * Full reference: docs/coop-api.md in https://github.com/multiplayereverything/ruffle-together
     *
     * Every call is safe outside Ruffle Together (plain Flash Player or plain Ruffle): Coop.active() is
     * false, one player, slot 0, mouse calls return NaN (use the game's own mouse), and calls that change
     * something do nothing.
     */
    public class Coop {
        // each group is made on first use, so the files compile in any order
        private static var _players:CoopPlayers;
        private static var _input:CoopInput;
        private static var _mouse:CoopMouse;
        private static var _zones:CoopZones;
        private static var _screens:CoopScreens;
        private static var _camera:CoopCamera;
        private static var _view:CoopView;
        private static var _room:CoopRoom;

        public static function get players():CoopPlayers {
            if (_players == null) _players = new CoopPlayers();
            return _players;
        }

        public static function get input():CoopInput {
            if (_input == null) _input = new CoopInput();
            return _input;
        }

        public static function get mouse():CoopMouse {
            if (_mouse == null) _mouse = new CoopMouse();
            return _mouse;
        }

        public static function get zones():CoopZones {
            if (_zones == null) _zones = new CoopZones();
            return _zones;
        }

        public static function get screens():CoopScreens {
            if (_screens == null) _screens = new CoopScreens();
            return _screens;
        }

        public static function get camera():CoopCamera {
            if (_camera == null) _camera = new CoopCamera();
            return _camera;
        }

        public static function get view():CoopView {
            if (_view == null) _view = new CoopView();
            return _view;
        }

        public static function get room():CoopRoom {
            if (_room == null) _room = new CoopRoom();
            return _room;
        }


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

        /** A raw API call by its name ("coop.camera", ...); undefined outside a session. */
        public static function call(name:String, ...args):* {
            if (!active()) {
                return undefined;
            }
            return ExternalInterface.call.apply(null, [name].concat(args));
        }
    }
}
