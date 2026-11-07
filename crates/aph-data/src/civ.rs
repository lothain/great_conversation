//! `civilizations.geojson`: rough civilization extents with a core and a fading halo.

use serde::{Deserialize, Serialize};

use crate::{
    edtf::Edtf,
    geo::Position,
    ids::CivId,
    vocab::{Confidence, ReligionFamily},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Civilizations {
    #[serde(rename = "type")]
    pub kind: FeatureCollectionType,
    pub meta: Meta,
    pub features: Vec<Feature>,
}

/// The `GeoJSON` `"type": "FeatureCollection"` marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureCollectionType {
    FeatureCollection,
}

/// The `GeoJSON` `"type": "Feature"` marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureType {
    Feature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub description: String,
    pub rendering: String,
    pub coords: String,
    pub dates: String,
    pub religions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    #[serde(rename = "type")]
    pub kind: FeatureType,
    /// The core extent, drawn at full opacity.
    pub geometry: Geometry,
    pub properties: Properties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
pub enum Geometry {
    Polygon(Polygon),
    MultiPolygon(Vec<Polygon>),
}

/// Bare coordinates of either geometry type (used for the halo).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Shape {
    Polygon(Polygon),
    MultiPolygon(Vec<Polygon>),
}

/// Outer ring first, then holes.
pub type Polygon = Vec<Ring>;

/// A closed ring: first position equals the last.
pub type Ring = Vec<Position>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Properties {
    pub civ: CivId,
    pub name: String,
    /// The snapshot date.
    pub when: Edtf,
    /// The interval this snapshot represents.
    pub valid: Edtf,
    /// Opacity falls linearly from 1 at the core edge to 0 at the halo edge.
    pub halo: Shape,
    pub religions: Vec<ReligionShare>,
    pub conf: Confidence,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReligionShare {
    pub family: ReligionFamily,
    pub share: f64,
}

impl Geometry {
    #[must_use]
    pub fn polygons(&self) -> &[Polygon] {
        match self {
            Self::Polygon(p) => std::slice::from_ref(p),
            Self::MultiPolygon(ps) => ps,
        }
    }
}

impl Shape {
    #[must_use]
    pub fn polygons(&self) -> &[Polygon] {
        match self {
            Self::Polygon(p) => std::slice::from_ref(p),
            Self::MultiPolygon(ps) => ps,
        }
    }

    /// Point-in-polygon against outer rings, ignoring holes.
    #[must_use]
    pub fn contains(&self, p: Position) -> bool {
        self.polygons().iter().filter_map(|poly| poly.first()).any(|ring| ring_contains(ring, p))
    }
}

/// Ray casting.
#[must_use]
pub fn ring_contains(ring: &[Position], p: Position) -> bool {
    let (x, y) = p.xy();
    ring.windows(2).fold(false, |inside, w| {
        let ((x1, y1), (x2, y2)) = (w[0].xy(), w[1].xy());
        if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 { !inside } else { inside }
    })
}

/// Twice the signed area; positive for counter-clockwise rings.
#[must_use]
pub fn signed_area2(ring: &[Position]) -> f64 {
    ring.windows(2)
        .map(|w| {
            let ((x1, y1), (x2, y2)) = (w[0].xy(), w[1].xy());
            x1 * y2 - x2 * y1
        })
        .sum()
}

/// True if any two non-adjacent edges of the ring cross.
#[must_use]
pub fn self_intersects(ring: &[Position]) -> bool {
    let edges: Vec<_> = ring.windows(2).map(|w| (w[0].xy(), w[1].xy())).collect();
    let n = edges.len();
    (0..n).any(|i| {
        ((i + 2)..n)
            .filter(|&j| !(i == 0 && j == n - 1))
            .any(|j| segments_cross(edges[i], edges[j]))
    })
}

fn segments_cross((a, b): ((f64, f64), (f64, f64)), (c, d): ((f64, f64), (f64, f64))) -> bool {
    let orient = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2) = (orient(c, d, a), orient(c, d, b));
    let (d3, d4) = (orient(a, b, c), orient(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}
