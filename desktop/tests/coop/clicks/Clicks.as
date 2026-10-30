package {
    import flash.display.Sprite;
    import flash.events.MouseEvent;

    /**
     * Test SWF for friends' clicks: a button (button mode, hand cursor) at 100,100 100x60 traces
     * "button" when clicked; any press anywhere traces "down X,Y" (stage pixels).
     * Build: java -cp tools/asc/asc.jar macromedia.asc.embedding.Main -AS3 -strict
     *   -import <playerglobal_import.abc> -swf Clicks,400,300,30 Clicks.as
     */
    public class Clicks extends Sprite {
        public function Clicks() {
            var button:Sprite = new Sprite();
            button.graphics.beginFill(0x3070F0);
            button.graphics.drawRect(0, 0, 100, 60);
            button.graphics.endFill();
            button.x = 100;
            button.y = 100;
            button.buttonMode = true;
            button.addEventListener(MouseEvent.CLICK, function (e:MouseEvent):void { trace("button"); });
            addChild(button);
            graphics.beginFill(0xF4F0E6);
            graphics.drawRect(0, 0, 400, 300);
            graphics.endFill();
            stage.addEventListener(MouseEvent.MOUSE_DOWN, function (e:MouseEvent):void {
                trace("down " + Math.round(stage.mouseX) + "," + Math.round(stage.mouseY));
            });
        }
    }
}
