//! Typed model, validation and tooling for the aphorisms dataset:
//! `aphorisms.json` (entries and authors), `graph.json` (who responds to what),
//! and `civilizations.geojson` (civilization extents over time).

pub mod check;
pub mod civ;
pub mod collection;
pub mod edtf;
pub mod geo;
pub mod gloss;
pub mod graph;
pub mod portraits;
pub mod schedule;
pub mod ids;
pub mod mentions;
pub mod store;
pub mod topojson;
pub mod vocab;
#[cfg(feature = "cli")]
pub mod wiki;
