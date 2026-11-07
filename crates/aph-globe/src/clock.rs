//! The year being shown, and the `Clock` uniform every material reads.
//!
//! Visibility by time is decided on the GPU (see `aph_scene::time` for the same rules in
//! Rust), so changing the year only rewrites one small uniform per material.

use aph_scene::time::Clock;
use bevy::{prelude::*, render::render_resource::ShaderType};

pub const FIRST_YEAR: f32 = -1000.0;
pub const LAST_YEAR: f32 = 2026.0;

/// What the globe is showing. Set by the host (HTML time bar, or the dev keys).
#[derive(Resource, Debug, Clone, Copy)]
pub struct ShownTime(pub Clock);

impl Default for ShownTime {
    fn default() -> Self {
        Self(Clock::at(FIRST_YEAR))
    }
}

/// GPU layout of [`Clock`] plus wall-clock seconds for animation.
#[derive(ShaderType, Debug, Clone, Copy, Default)]
pub struct GpuClock {
    pub year: f32,
    pub window: f32,
    pub seconds: f32,
    pub reveal: f32,
    /// Physical pixels per logical pixel, so lines and dots keep their size on any display.
    pub px_ratio: f32,
    // Uniform arrays need 16-byte strides, so pad with scalars.
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

/// A material that reads the clock.
pub trait Clocked: Material {
    fn clock_mut(&mut self) -> &mut GpuClock;
}

/// Upload the clock to every instance of `M`.
pub fn sync<M: Clocked>(
    time: Res<Time>,
    shown: Res<ShownTime>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    handles: Query<&MeshMaterial3d<M>>,
    mut materials: ResMut<Assets<M>>,
) {
    let Clock { year, window, reveal } = shown.0;
    let px_ratio = windows.single().map_or(1.0, Window::scale_factor);
    let gpu = GpuClock { year, window, seconds: time.elapsed_secs(), reveal, px_ratio, _pad0: 0.0, _pad1: 0.0, _pad2: 0.0 };
    for handle in &handles {
        if let Some(mut m) = materials.get_mut(&handle.0) {
            *m.clock_mut() = gpu;
        }
    }
}
