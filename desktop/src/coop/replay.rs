//! `--coop-replay`: runs a game headless from a script and records its fingerprint every frame.
//! Two runs with the same script must give the same fingerprints however fast or slow the PC is;
//! that is the guarantee Ruffle Together's lockstep rests on.

use crate::coop::CoopOpt;
use crate::coop::frame::Stepper;
use crate::coop::keymap::Game;
use crate::coop::script::Script;
use crate::coop::storage::SessionStorage;
use anyhow::{Context, Result, anyhow};
use rand::RngExt;
use ruffle_core::backend::locale::DefaultLocaleBackend;
use ruffle_core::backend::navigator::NullNavigatorBackend;
use ruffle_core::frame_clock::{FrameClock, FrameClockLocale};
use ruffle_core::tag_utils::movie_from_path;
use ruffle_core::{Player, PlayerBuilder};
use ruffle_render_wgpu::backend::{WgpuRenderBackend, create_wgpu_instance, request_adapter_and_device};
use ruffle_render_wgpu::descriptors::Descriptors;
use ruffle_render_wgpu::target::TextureTarget;
use ruffle_render_wgpu::wgpu;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Replay<'a> {
    pub swf: &'a Path,
    pub script: &'a Script,
    pub game: Game,
    pub frames: u32,
    pub jitter_ms: u64,
    /// false: the real clock, as upstream Ruffle (the control run).
    pub frame_clock: bool,
    /// The date at frame 0 (it also seeds Math.random).
    pub start_unix_ms: i64,
    /// Frames to save as `<shot_base>.<frame>.png`.
    pub shots: &'a [u32],
    pub shot_base: Option<&'a Path>,
}

impl Replay<'_> {
    /// The game's fingerprint after every frame.
    pub fn run(&self) -> Result<Vec<u64>> {
        let clock = Arc::new(Mutex::new(FrameClock::new(self.start_unix_ms)));
        let player = headless_player(
            self.swf,
            self.frame_clock.then_some(clock),
            SessionStorage::default(),
        )?;
        let mut player = player.lock().map_err(|_| anyhow!("player lock"))?;

        let mut stepper = Stepper::new(self.game.keys(), self.game.max_players(), 0);
        let mut rng = rand::rng();
        let mut prints = Vec::with_capacity(self.frames as usize);
        for frame in 0..self.frames {
            let f = frame as usize;
            let inputs: Vec<_> =
                (0..self.game.max_players() as usize).map(|p| self.script.input(f, p)).collect();
            stepper.step(&mut player, &inputs);
            prints.push(player.coop_fingerprint());
            if let Ok(at) = std::env::var("COOP_DESCRIBE_AT")
                && at.split(',').any(|f| f.trim() == frame.to_string())
                && let Some(base) = self.shot_base
            {
                std::fs::write(base.with_extension(format!("{frame}.objects.txt")), player.coop_describe())?;
            }
            if self.shots.contains(&frame)
                && let Some(base) = self.shot_base
            {
                save_shot(&mut player, &base.with_extension(format!("{frame}.png")))?;
            }
            if self.jitter_ms > 0 {
                std::thread::sleep(Duration::from_millis(rng.random_range(0..=self.jitter_ms)));
            }
        }
        Ok(prints)
    }
}

/// A windowless player for `swf` (the first `Stepper::step` loads it fully). With a frame clock it
/// runs as in a session (getTimer, Date and Math.random follow the clock); without, as upstream.
pub fn headless_player(
    swf: &Path,
    clock: Option<Arc<Mutex<FrameClock>>>,
    storage: SessionStorage,
) -> Result<Arc<Mutex<Player>>> {
    let movie = movie_from_path(swf, None).map_err(|e| anyhow!("{e}"))?;
    let (width, height) = (
        movie.width().to_pixels().round() as u32,
        movie.height().to_pixels().round() as u32,
    );
    let mut builder = PlayerBuilder::new()
        .with_movie(movie)
        .with_navigator(NullNavigatorBackend::new())
        .with_storage(Box::new(storage))
        .with_viewport_dimensions(width, height, 1.0)
        .with_autoplay(true)
        // always a real renderer, as in a session: games draw clips into bitmaps and change
        // what is on stage when that works (Bad Ice-Cream's title screen does), so a renderer
        // that can't draw takes a different path
        .with_renderer(offscreen_renderer(width, height)?);
    builder = match &clock {
        Some(clock) => builder.with_locale(FrameClockLocale::new(clock.clone())),
        None => builder.with_locale(DefaultLocaleBackend::new()),
    };
    let player = builder.build();
    player.lock().map_err(|_| anyhow!("player lock"))?.set_frame_clock(clock);
    Ok(player)
}

fn offscreen_renderer(width: u32, height: u32) -> Result<WgpuRenderBackend<TextureTarget>> {
    let backends = wgpu::Backends::PRIMARY;
    let instance = create_wgpu_instance(backends, wgpu::BackendOptions::default(), None);
    let (adapter, device, queue) = futures::executor::block_on(request_adapter_and_device(
        backends,
        &instance,
        None,
        wgpu::PowerPreference::LowPower,
    ))
    .map_err(|e| anyhow!("{e}"))?;
    let descriptors = Arc::new(Descriptors::new(instance, adapter, device, queue));
    let target = TextureTarget::new(&descriptors.device, (width, height)).map_err(|e| anyhow!("{e}"))?;
    WgpuRenderBackend::new(descriptors, target).map_err(|e| anyhow!("{e}"))
}

fn save_shot(player: &mut Player, path: &Path) -> Result<()> {
    player.render();
    let renderer = <dyn std::any::Any>::downcast_mut::<WgpuRenderBackend<TextureTarget>>(
        player.renderer_mut(),
    )
    .context("screenshots need the offscreen renderer")?;
    let image = renderer.capture_frame().context("capture failed")?;
    image.save(path)?;
    Ok(())
}

/// The `--coop-replay` command line.
pub fn run_cli(opt: &CoopOpt) -> Result<()> {
    let swf = opt.coop_replay.as_deref().context("--coop-replay needs a SWF")?;
    let script = match &opt.coop_script {
        Some(path) => Script::parse(&std::fs::read_to_string(path)?).map_err(|e| anyhow!(e))?,
        None => Script::parse("").map_err(|e| anyhow!(e))?,
    };
    let out = opt.coop_out.as_deref().context("--coop-out is required")?;
    let prints = Replay {
        swf,
        script: &script,
        game: opt.game()?,
        frames: opt.coop_frames,
        jitter_ms: opt.coop_jitter_ms,
        frame_clock: !opt.coop_no_clock,
        start_unix_ms: opt.coop_start_ms,
        shots: &opt.coop_shot,
        shot_base: Some(out),
    }
    .run()?;
    let text: String = prints
        .iter()
        .enumerate()
        .map(|(frame, print)| format!("{frame} {print:016x}\n"))
        .collect();
    std::fs::write(out, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the game file (not in the repo): COOP_TEST_SWF=<path to BadIceCream_coop.swf>.
    #[test]
    fn bad_ice_cream_runs_the_same_every_time() {
        let Ok(swf) = std::env::var("COOP_TEST_SWF") else {
            eprintln!("skipping: COOP_TEST_SWF not set");
            return;
        };
        let script =
            Script::parse(include_str!("../../tests/coop/bic-two-players.txt")).expect("script");
        let run = |jitter_ms, start_unix_ms| {
            Replay {
                swf: Path::new(&swf),
                script: &script,
                game: Game::built_in("bad-ice-cream").expect("built in"),
                frames: 1000,
                jitter_ms,
                frame_clock: true,
                start_unix_ms,
                shots: &[],
                shot_base: None,
            }
            .run()
            .expect("replay")
        };
        let steady = run(0, 1_700_000_000_000);
        let jittery = run(10, 1_700_000_000_000);
        assert_eq!(steady, jittery, "the PC's speed changed the game");

        // control: another start date seeds Math.random differently, and the fingerprint sees it
        let other_seed = run(0, 1_700_000_000_001);
        assert_ne!(steady, other_seed, "the fingerprint missed a different game");
    }
}
