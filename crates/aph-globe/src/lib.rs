//! The aphorisms globe renderer, as a set of Bevy plugins.
//!
//! The host (the web page, or the native dev binary) drives it with [`host::HostCommand`]s and
//! listens to [`host::HostEvent`]s. The renderer draws; it owns no widgets and no playback.

pub mod atmosphere;
pub mod camera;
pub mod clock;
#[cfg(not(target_arch = "wasm32"))]
pub mod dev;
pub mod dots;
pub mod globe;
pub mod host;
pub mod interact;
pub mod overlay;
pub mod scene;
pub mod threads;

use aph_scene::atlas::Atlas;
use bevy::{asset::embedded_asset, prelude::*};

/// Everything needed to draw the globe for `atlas` (but not the window or `DefaultPlugins`).
pub struct GlobePlugins {
    scene: std::sync::Mutex<Option<scene::Scene>>,
}

impl GlobePlugins {
    /// `baked_coasts`: the PNG from `aph-bake`, to skip computing the coastline field at startup.
    #[must_use]
    pub fn new(atlas: Atlas, baked_coasts: Option<Vec<u8>>) -> Self {
        Self { scene: std::sync::Mutex::new(Some(scene::Scene { atlas, baked_coasts })) }
    }
}

impl Plugin for GlobePlugins {
    fn build(&self, app: &mut App) {
        // Imported by the others, so it must be loaded (registered) rather than just embedded.
        bevy::shader::load_shader_library!(app, "shaders/common.wesl");
        embedded_asset!(app, "shaders/globe.wesl");
        embedded_asset!(app, "shaders/atmosphere.wesl");
        embedded_asset!(app, "shaders/threads.wesl");
        embedded_asset!(app, "shaders/dots.wesl");

        let scene = self.scene.lock().expect("not poisoned").take().expect("GlobePlugins built once");
        app.insert_resource(ClearColor(Color::srgb_u8(0x2a, 0x25, 0x20)))
            .insert_resource(scene)
            .init_resource::<clock::ShownTime>()
            .add_plugins((
                host::HostPlugin,
                overlay::OverlayPlugin,
                globe::GlobePlugin,
                atmosphere::AtmospherePlugin,
                threads::ThreadsPlugin,
                dots::DotsPlugin,
                camera::CameraPlugin,
                interact::InteractPlugin,
            ));
    }
}
