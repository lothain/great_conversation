//! Everything the aphorisms globe needs that doesn't depend on a renderer:
//!
//! - [`atlas`]: the dataset resolved into located aphorisms, ideas, threads and civilizations
//! - [`sphere`]: latitude/longitude geometry and great-circle arcs
//! - [`time`]: the time-visibility rules the shaders implement
//! - [`buffers`]: vertex buffers for the batched thread and dot meshes
//! - [`overlay`]: the civilization texture with distance-field halos
//! - [`pick`]: pointer rays to globe positions and items
//! - [`palette`]: colours shared with the HTML widgets

pub mod atlas;
pub mod buffers;
pub mod export;
pub mod overlay;
pub mod palette;
pub mod pick;
pub mod regions;
pub mod sphere;
pub mod time;
