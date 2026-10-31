package {
    /** Coop.room: the session: the host's settings, what happened, what this build can do. */
    public class CoopRoom {
        /** The API version (0 outside a session). */
        public function version():int { return Coop.active() ? int(Coop.call("coop.version")) : 0; }

        /** True if this build has the feature ("camera", "pointers", "clicker", ...). */
        public function has(feature:String):Boolean { return Coop.call("coop.has", feature) == true; }

        /** The host's setting (--coop-option NAME=VALUE), or null. */
        public function option(name:String):String {
            var o:* = Coop.call("coop.option", name);
            return o == undefined ? null : String(o);
        }

        /** Every host setting, as an object (empty outside a session). */
        public function options():Object {
            var o:* = Coop.call("coop.options");
            return o == undefined ? {} : o;
        }

        /** What happened since the last call, oldest first: ["join 0", "leave 2", "chat 1 gg", ...]. Read it once per frame. */
        public function events():Array {
            var e:* = Coop.call("coop.events");
            return e == undefined ? [] : e as Array;
        }
    }
}
