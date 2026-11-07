//! The resolved dataset, shared with every renderer system.

use aph_scene::atlas::Atlas;
use bevy::prelude::*;

#[derive(Resource, Debug)]
pub struct Scene {
    pub atlas: Atlas,
    /// `data/baked/coasts.png` (from `aph-bake`), if available; otherwise the coastline
    /// field is computed at startup.
    pub baked_coasts: Option<Vec<u8>>,
}
