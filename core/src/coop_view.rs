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
}
