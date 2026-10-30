package {
    import flash.display.Sprite;
    import flash.events.MouseEvent;

    /**
     * Test SWF for friends' clicks: a button (button mode, hand cursor) at 100,100 100x60 traces
     * "button" when clicked and opens a pop-up (_root.popup, 90,90 200x120, over the button) whose own
     * button (at 220,170 60x30) traces "popup" and closes it; any press anywhere traces "down X,Y".
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
            addChild(button);
            var popup:Sprite = new Sprite();
            popup.name = "popup";
            popup.graphics.beginFill(0x404040);
            popup.graphics.drawRect(90, 90, 200, 120);
            popup.graphics.endFill();
            popup.visible = false;
            var close:Sprite = new Sprite();
            close.graphics.beginFill(0xE04040);
            close.graphics.drawRect(220, 170, 60, 30);
            close.graphics.endFill();
            close.buttonMode = true;
            close.addEventListener(MouseEvent.CLICK, function (e:MouseEvent):void { trace("popup"); popup.visible = false; });
            popup.addChild(close);
            addChild(popup);
            button.addEventListener(MouseEvent.CLICK, function (e:MouseEvent):void { trace("button"); popup.visible = true; });
            graphics.beginFill(0xF4F0E6);
            graphics.drawRect(0, 0, 400, 300);
            graphics.endFill();
            stage.addEventListener(MouseEvent.MOUSE_DOWN, function (e:MouseEvent):void {
                trace("down " + Math.round(stage.mouseX) + "," + Math.round(stage.mouseY));
            });
        }
    }
}
