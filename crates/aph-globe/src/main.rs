//! Native: `cargo run -p aph-globe [-- path/to/data]` opens the globe with keyboard dev controls.
//! Web: built by Trunk from `web/`; the page calls the exports in [`web`] instead of `main`.

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use std::path::PathBuf;

    use aph_data::store::Dataset;
    use aph_globe::{GlobePlugins, dev::DevControlsPlugin};
    use aph_scene::atlas::Atlas;
    use bevy::prelude::*;

    let dir = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data")), PathBuf::from);
    let atlas = Atlas::build(&Dataset::load(&dir)?);
    let baked_coasts = std::fs::read(dir.join("baked/coasts.png")).ok();
    for missing in &atlas.unplaced {
        eprintln!("unplaced thread: {missing}");
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "The Word About Words".into(),
                // `APH_SIZE=1280x720`: a fixed physical size at scale 1, for reproducible snapshots.
                resolution: fixed_size().unwrap_or_default(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((GlobePlugins::new(atlas, baked_coasts), DevControlsPlugin))
        .run();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn fixed_size() -> Option<bevy::window::WindowResolution> {
    let size = std::env::var("APH_SIZE").ok()?;
    let (w, h) = size.split_once('x')?;
    Some(bevy::window::WindowResolution::new(w.parse().ok()?, h.parse().ok()?).with_scale_factor_override(1.0))
}

/// On the web the page starts the globe by calling `web::start`.
#[cfg(target_arch = "wasm32")]
fn main() {}
