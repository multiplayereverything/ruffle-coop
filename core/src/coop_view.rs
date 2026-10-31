//! Ruffle Together's per-PC view: a camera and HUD clips that change only the picture each PC
//! draws, never the game (nothing here moves a display object or touches what the fingerprint
//! sees).

use crate::context::UpdateContext;
use crate::display_object::{BoundsMode, DisplayObject, TDisplayObject, TDisplayObjectContainer};
use ruffle_render::matrix::Matrix;
use swf::Twips;

/// The clip at `path`: `_root` / `_level0` / `root`, then instance names joined by dots
/// (`_root.hud.score`). A path without a root starts at the root (`hud.score`).
pub fn find<'gc>(context: &UpdateContext<'gc>, path: &str) -> Option<DisplayObject<'gc>> {
    let mut object = context.stage.root_clip()?;
    for (i, name) in path.split('.').enumerate() {
        if i == 0 && matches!(name, "_root" | "_level0" | "root") {
            continue;
        }
        let case_sensitive = object.swf_version() >= 7;
        object = object
            .as_container()?
            .child_by_name(&ruffle_wstr::WString::from_utf8(name), case_sensitive)?;
    }
    Some(object)
}

/// The path of `object` in the form `find` reads: `_root`, then instance names joined by dots.
pub fn path_of<'gc>(context: &UpdateContext<'gc>, object: DisplayObject<'gc>) -> String {
    let root = context.stage.root_clip();
    let mut names = vec![];
    let mut current = Some(object);
    while let Some(o) = current {
        if root.is_some_and(|root| DisplayObject::ptr_eq(root, o)) {
            break;
        }
        names.push(o.name().map(|n| n.to_utf8_lossy().into_owned()).unwrap_or_default());
        current = o.parent();
    }
    names.push("_root".into());
    names.reverse();
    names.join(".")
}

/// Whether the clip at `path` is on show (it exists and it and its parents are visible).
pub fn shown(context: &UpdateContext<'_>, path: &str) -> bool {
    let Some(object) = find(context, path) else {
        return false;
    };
    let mut current = Some(object);
    while let Some(o) = current {
        if !o.visible() {
            return false;
        }
        current = o.parent();
    }
    true
}

/// Whether the stage point (x, y) (twips) is on the clip at `path` (in its bounds).
pub fn covers(context: &UpdateContext<'_>, path: &str, x: Twips, y: Twips) -> bool {
    find(context, path).is_some_and(|object| {
        let bounds = object.world_bounds(BoundsMode::Engine);
        x >= bounds.x_min && x < bounds.x_max && y >= bounds.y_min && y < bounds.y_max
    })
}

/// Whether this PC draws the clip at `path` (another player's private screen isn't drawn).
pub fn set_drawn(context: &UpdateContext<'_>, path: &str, drawn: bool) -> bool {
    find(context, path).map(|object| object.base().set_coop_not_drawn(!drawn)).is_some()
}

/// Takes the clip at `path` out of mouse picking (or puts it back): a player's mouse goes
/// through another player's private screen. Returns whether it was visible, to put back.
pub fn set_pickable(context: &UpdateContext<'_>, path: &str, pickable: bool) -> Option<bool> {
    let object = find(context, path)?;
    let was = object.visible();
    object.base().set_visible_for_picking(pickable);
    Some(was)
}

/// Marks the clip at `path` as HUD (drawn without the camera); false if there is none.
pub fn set_hud(context: &mut UpdateContext<'_>, path: &str) -> bool {
    find(context, path).map(|object| object.set_coop_hud(true)).is_some()
}

/// Where a label for the clip at `path` goes: the middle of its top edge, in stage twips.
pub fn label_anchor(context: &UpdateContext<'_>, path: &str) -> Option<(Twips, Twips)> {
    let object = find(context, path)?;
    if !object.visible() {
        return None;
    }
    let bounds = object.world_bounds(BoundsMode::Engine);
    Some(((bounds.x_min + bounds.x_max) / 2, bounds.y_min))
}

/// The camera that shows the stage point (x, y), in stage pixels, at the middle of a stage
/// `width` x `height` pixels, `zoom` times bigger.
pub fn camera_matrix(width: f64, height: f64, x: f64, y: f64, zoom: f64) -> Matrix {
    let to_twips = |pixels: f64| Twips::from_pixels(pixels);
    Matrix::translate(to_twips(width / 2.0), to_twips(height / 2.0))
        * Matrix::scale(zoom as f32, zoom as f32)
        * Matrix::translate(to_twips(-x), to_twips(-y))
}

/// The matrix a HUD clip is drawn with. Everything is drawn at `view * camera * parent * matrix`
/// (`parent`: the clip's parent in stage coordinates); a HUD clip must land where it would with no
/// camera, `view * parent * own`, also when the game moves its parents (Intrusion scrolls by
/// moving `_root`). None when the camera or the parent can't be undone.
pub fn hud_matrix(camera: Matrix, parent: Matrix, own: Matrix) -> Option<Matrix> {
    Some(parent.inverse()? * camera.inverse()? * parent * own)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Point;

    #[test]
    fn the_camera_centres_its_point_and_zooms_around_it() {
        let camera = camera_matrix(800.0, 600.0, 1000.0, 200.0, 2.0);
        let at = |x: f64, y: f64| camera * Point::new(Twips::from_pixels(x), Twips::from_pixels(y));
        assert_eq!(at(1000.0, 200.0), Point::new(Twips::from_pixels(400.0), Twips::from_pixels(300.0)));
        assert_eq!(at(1010.0, 200.0), Point::new(Twips::from_pixels(420.0), Twips::from_pixels(300.0)));
        let still = camera_matrix(800.0, 600.0, 400.0, 300.0, 1.0);
        assert_eq!(still, Matrix::IDENTITY, "centred at the middle, zoom 1: no change");
    }

    /// Intrusion moves `_root` to scroll (about 6000 px) and keeps `_root.ui` at minus that: with a
    /// zoomed camera the HUD still lands where it would with no camera at all.
    #[test]
    fn a_hud_inside_a_moved_parent_lands_where_it_would_without_a_camera() {
        let px = |pixels: f64| Twips::from_pixels(pixels);
        let camera = camera_matrix(720.0, 540.0, 360.0, 270.0, 1.5);
        let root = Matrix::translate(px(6000.0), px(-40.0));
        let ui = Matrix::translate(px(-6000.0), px(40.0)) * Matrix::translate(px(12.0), px(8.0));
        let drawn = camera * root * hud_matrix(camera, root, ui).expect("invertible");
        let corner = Point::new(Twips::ZERO, Twips::ZERO);
        assert_eq!(drawn * corner, root * ui * corner);
        assert_eq!(drawn * corner, Point::new(px(12.0), px(8.0)), "the HUD's own spot on the stage");
        // the old undo, inside the parent, put it zoom-1 times the root's offset away
        let old = camera * root * camera.inverse().expect("invertible") * ui;
        assert_ne!(old * corner, root * ui * corner);
        // a parent at the stage's origin: the same as undoing the camera alone
        let at_origin = hud_matrix(camera, Matrix::IDENTITY, ui).expect("invertible");
        assert_eq!(at_origin, camera.inverse().expect("invertible") * ui);
    }
}
