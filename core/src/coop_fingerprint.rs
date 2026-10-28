//! Ruffle Together's sync check: a fingerprint of the game that is equal on every PC exactly
//! when the games are in the same state (clock, random numbers, and what is on stage).

use crate::context::UpdateContext;
use crate::display_object::{DisplayObject, TDisplayObject, TDisplayObjectContainer};

/// FNV-1a 64: fixed constants, so the hash is the same on every PC and build
/// (std's hasher is seeded randomly per process).
pub struct Fnv(u64);

impl Fnv {
    pub fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub fn add(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.0 ^= byte as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv {
    fn default() -> Self {
        Self::new()
    }
}

pub fn fingerprint(context: &mut UpdateContext<'_>) -> u64 {
    let mut hash = Fnv::new();
    hash.add(context.elapsed_since_start().as_millis() as u64);
    hash.add(context.rng.state() as u64);
    add_object(context.stage.into(), &mut hash);
    hash.finish()
}

/// What the fingerprint covers, one line per object, to find what differs after a desync.
pub fn describe(context: &mut UpdateContext<'_>) -> String {
    let mut out = format!(
        "clock {} ms, rng {:08x}\n",
        context.elapsed_since_start().as_millis(),
        context.rng.state()
    );
    describe_object(context.stage.into(), 0, &mut out);
    out
}

fn describe_object(object: DisplayObject<'_>, indent: usize, out: &mut String) {
    let frame = object.as_movie_clip().map(|clip| clip.current_frame());
    out.push_str(&format!(
        "{:indent$}depth {} id {} name {} at ({}, {}) frame {:?} visible {}\n",
        "",
        object.depth(),
        object.id(),
        object.name().map(|n| n.to_string()).unwrap_or_default(),
        object.x().get(),
        object.y().get(),
        frame,
        object.visible(),
    ));
    if let Some(container) = object.as_container() {
        for child in container.iter_render_list() {
            describe_object(child, indent + 2, out);
        }
    }
}

/// Every object on stage, depth first: where it is, what it is, which frame it shows.
fn add_object(object: DisplayObject<'_>, hash: &mut Fnv) {
    hash.add(object.depth() as i64 as u64);
    hash.add(object.id() as u64);
    hash.add(object.x().get() as i64 as u64);
    hash.add(object.y().get() as i64 as u64);
    hash.add(object.visible() as u64);
    if let Some(clip) = object.as_movie_clip() {
        hash.add(clip.current_frame() as u64);
    }
    if let Some(container) = object.as_container() {
        hash.add(container.num_children() as u64);
        for child in container.iter_render_list() {
            add_object(child, hash);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_is_stable_and_order_sensitive() {
        let mut a = Fnv::new();
        a.add(1);
        a.add(2);
        let mut b = Fnv::new();
        b.add(1);
        b.add(2);
        let mut c = Fnv::new();
        c.add(2);
        c.add(1);
        assert_eq!(a.finish(), b.finish());
        assert_ne!(a.finish(), c.finish());
        // FNV-1a 64 of eight zero bytes: the same on every PC and every build
        let mut d = Fnv::new();
        d.add(0);
        assert_eq!(d.finish(), 0xa8c7_f832_281a_39c5);
    }
}
