//! Loading and saving the three data files.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

use crate::{civ::Civilizations, collection::Collection, graph::Graph, mentions::Mentions, portraits::Portraits, schedule::CuratorSchedule};

pub struct Dataset {
    pub dir: PathBuf,
    pub collection: Collection,
    pub graph: Graph,
    pub civilizations: Civilizations,
    /// Optional: absent file means no portraits.
    pub portraits: Portraits,
    /// Optional: utterance-to-utterance callbacks.
    pub mentions: Mentions,
    /// Optional: when the curator's replies were written.
    pub schedule: Option<CuratorSchedule>,
    /// Optional: land polygons (Natural Earth, cached in `data/`), for coastlines.
    pub land: Vec<crate::civ::Polygon>,
}

pub const COLLECTION: &str = "aphorisms.json";
pub const GRAPH: &str = "graph.json";
pub const CIVILIZATIONS: &str = "civilizations.geojson";
pub const PORTRAITS: &str = "portraits.json";
pub const MENTIONS: &str = "mentions.json";
pub const SCHEDULE: &str = "curator_schedule.json";
pub const LAND: &str = "land-50m.json";

/// Land polygons from a cached `TopoJSON` file's `land` object.
pub fn land_from_json(text: &str) -> Result<Vec<crate::civ::Polygon>> {
    let topo: crate::topojson::Topology = serde_json::from_str(text).context("parsing land TopoJSON")?;
    Ok(topo.polygons("land")?)
}

/// Read an optional file: `None` when absent.
fn read_optional<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if path.exists() { read(path).map(Some) } else { Ok(None) }
}

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

impl Dataset {
    pub fn load(dir: &Path) -> Result<Self> {
        Ok(Self {
            dir: dir.to_owned(),
            collection: read(&dir.join(COLLECTION))?,
            graph: read(&dir.join(GRAPH))?,
            civilizations: read(&dir.join(CIVILIZATIONS))?,
            portraits: read_optional(&dir.join(PORTRAITS))?.unwrap_or_default(),
            mentions: read_optional(&dir.join(MENTIONS))?.unwrap_or_default(),
            schedule: read_optional(&dir.join(SCHEDULE))?,
            land: match fs::read_to_string(dir.join(LAND)) {
                Ok(text) => land_from_json(&text)?,
                Err(_) => Vec::new(),
            },
        })
    }

    /// From the three files' contents, for hosts without a filesystem (the browser).
    pub fn from_json(collection: &str, graph: &str, civilizations: &str, portraits: Option<&str>, land: Option<&str>) -> Result<Self> {
        Ok(Self {
            dir: PathBuf::new(),
            collection: serde_json::from_str(collection).context("parsing aphorisms.json")?,
            graph: serde_json::from_str(graph).context("parsing graph.json")?,
            civilizations: serde_json::from_str(civilizations).context("parsing civilizations.geojson")?,
            portraits: portraits.map(serde_json::from_str).transpose().context("parsing portraits.json")?.unwrap_or_default(),
            // The browser build doesn't use these yet.
            mentions: Mentions::default(),
            schedule: None,
            land: land.map(land_from_json).transpose()?.unwrap_or_default(),
        })
    }

    pub fn save(&self) -> Result<()> {
        write(&self.dir.join(COLLECTION), &self.collection)?;
        write(&self.dir.join(GRAPH), &self.graph)?;
        write(&self.dir.join(CIVILIZATIONS), &self.civilizations)?;
        if self.portraits.is_empty() { Ok(()) } else { write(&self.dir.join(PORTRAITS), &self.portraits) }
    }
}
