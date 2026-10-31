package {
    import flash.display.Shape;
    import flash.display.SimpleButton;
    import flash.display.Sprite;
    import flash.events.MouseEvent;
    import flash.external.ExternalInterface;

    /**
     * Test SWF for every player's own mouse (400x300):
     * - sprite button "a" (button mode) at 100,100 100x60: traces "over a" / "out a" on roll over and
     *   out, and "click a P X,Y" on click (P: coop.mouse.clicker, X,Y: stage.mouseX/Y in the handler);
     * - a draggable square "drag" at 100,200 40x40: press starts a drag, any release stops it and
     *   traces "dropped X,Y" (its position);
     * - a SimpleButton "states" at 250,100 100x60 with Up (blue), Over (red) and Down (green) art.
     * Build: java -cp tools/asc/asc.jar macromedia.asc.embedding.Main -AS3 -strict
     *   -import <playerglobal_import.abc> -swf Mice,400,300,30 Mice.as
     */
    public class Mice extends Sprite {
        public function Mice() {
            graphics.beginFill(0xF4F0E6);
            graphics.drawRect(0, 0, 400, 300);
            graphics.endFill();

            var a:Sprite = new Sprite();
            a.name = "a";
            a.graphics.beginFill(0x3070F0);
            a.graphics.drawRect(0, 0, 100, 60);
            a.graphics.endFill();
            a.x = 100;
            a.y = 100;
            a.buttonMode = true;
            a.addEventListener(MouseEvent.ROLL_OVER, function (e:MouseEvent):void { trace("over a"); });
            a.addEventListener(MouseEvent.ROLL_OUT, function (e:MouseEvent):void { trace("out a"); });
            a.addEventListener(MouseEvent.CLICK, function (e:MouseEvent):void {
                trace("click a " + ExternalInterface.call("coop.mouse.clicker") + " " + Math.round(stage.mouseX) + "," + Math.round(stage.mouseY));
            });
            addChild(a);

            var drag:Sprite = new Sprite();
            drag.name = "drag";
            drag.graphics.beginFill(0x30B050);
            drag.graphics.drawRect(0, 0, 40, 40);
            drag.graphics.endFill();
            drag.x = 100;
            drag.y = 200;
            drag.buttonMode = true;
            drag.addEventListener(MouseEvent.MOUSE_DOWN, function (e:MouseEvent):void { drag.startDrag(); });
            addChild(drag);
            stage.addEventListener(MouseEvent.MOUSE_UP, function (e:MouseEvent):void {
                if (drag.x != 100 || drag.y != 200) {
                    drag.stopDrag();
                    trace("dropped " + Math.round(drag.x) + "," + Math.round(drag.y));
                }
            });

            var states:SimpleButton = new SimpleButton(box(0x3070F0), box(0xE04040), box(0x30B050), box(0));
            states.name = "states";
            states.x = 250;
            states.y = 100;
            addChild(states);
        }

        private function box(colour:uint):Shape {
            var shape:Shape = new Shape();
            shape.graphics.beginFill(colour);
            shape.graphics.drawRect(0, 0, 100, 60);
            shape.graphics.endFill();
            return shape;
        }
    }
}
