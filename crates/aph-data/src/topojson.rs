//! Just enough `TopoJSON` to read polygon layers such as Natural Earth land
//! (`data/land-50m.json`, from the `world-atlas` package).
//!
//! `TopoJSON` stores each boundary once as an "arc" of quantized, delta-encoded points;
//! rings are lists of arc indices, where `!i` (i.e. `-i - 1`) means arc `i` reversed.

use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;

use crate::{
    civ::{Polygon, Ring},
    geo::Position,
};

#[derive(Debug, Deserialize)]
pub struct Topology {
    pub objects: BTreeMap<String, Object>,
    pub arcs: Vec<Vec<[i64; 2]>>,
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Transform {
    pub scale: [f64; 2],
    pub translate: [f64; 2],
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Object {
    GeometryCollection { geometries: Vec<Object> },
    Polygon { arcs: Vec<Vec<i64>> },
    MultiPolygon { arcs: Vec<Vec<Vec<i64>>> },
    #[serde(other)]
    Other,
}

#[derive(Debug, Error)]
pub enum TopoError {
    #[error("no object named `{0}`")]
    MissingObject(String),
    #[error("arc index {0} out of range")]
    BadArc(i64),
}

impl Topology {
    /// Arc `i` in longitude/latitude degrees: undo the delta encoding and the quantization.
    fn arc(&self, index: i64) -> Result<Vec<Position>, TopoError> {
        let (i, reversed) = if index < 0 { (!index, true) } else { (index, false) };
        let raw = usize::try_from(i).ok().and_then(|i| self.arcs.get(i)).ok_or(TopoError::BadArc(index))?;
        let mut points = Vec::with_capacity(raw.len());
        let (mut x, mut y) = (0i64, 0i64);
        for [dx, dy] in raw {
            let (lon, lat) = match self.transform {
                Some(t) => {
                    (x, y) = (x + dx, y + dy);
                    (x as f64 * t.scale[0] + t.translate[0], y as f64 * t.scale[1] + t.translate[1])
                }
                None => (*dx as f64, *dy as f64),
            };
            let clamped = [lon.clamp(-180.0, 180.0), lat.clamp(-90.0, 90.0)];
            points.push(Position::try_from(clamped).expect("clamped into range"));
        }
        if reversed {
            points.reverse();
        }
        Ok(points)
    }

    /// Stitch a ring from arcs; consecutive arcs share their joining point.
    fn ring(&self, arcs: &[i64]) -> Result<Ring, TopoError> {
        let mut ring: Ring = Vec::new();
        for &a in arcs {
            let points = self.arc(a)?;
            let skip = usize::from(!ring.is_empty());
            ring.extend(points.into_iter().skip(skip));
        }
        Ok(ring)
    }

    fn polygon(&self, rings: &[Vec<i64>]) -> Result<Polygon, TopoError> {
        rings.iter().map(|r| self.ring(r)).collect()
    }

    /// Every polygon of the named object.
    pub fn polygons(&self, object: &str) -> Result<Vec<Polygon>, TopoError> {
        let root = self.objects.get(object).ok_or_else(|| TopoError::MissingObject(object.to_owned()))?;
        let mut out = Vec::new();
        self.collect(root, &mut out)?;
        Ok(out)
    }

    fn collect(&self, object: &Object, out: &mut Vec<Polygon>) -> Result<(), TopoError> {
        match object {
            Object::GeometryCollection { geometries } => {
                for g in geometries {
                    self.collect(g, out)?;
                }
            }
            Object::Polygon { arcs } => out.push(self.polygon(arcs)?),
            Object::MultiPolygon { arcs } => {
                for p in arcs {
                    out.push(self.polygon(p)?);
                }
            }
            Object::Other => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = r#"{
        "type": "Topology",
        "transform": {"scale": [1, 1], "translate": [-10, -10]},
        "arcs": [[[0, 0], [20, 0], [0, 20]], [[20, 20], [-20, 0], [0, -20]]],
        "objects": {"land": {"type": "GeometryCollection", "geometries": [{"type": "Polygon", "arcs": [[0, 1]]}]}}
    }"#;

    #[test]
    fn decodes_quantized_delta_arcs_into_a_closed_ring() {
        let t: Topology = serde_json::from_str(SQUARE).unwrap();
        let polys = t.polygons("land").unwrap();
        let ring: Vec<_> = polys[0][0].iter().map(|p| p.xy()).collect();
        assert_eq!(ring, vec![(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0), (-10.0, -10.0)]);
    }

    #[test]
    fn negative_indices_reverse_arcs() {
        let t: Topology = serde_json::from_str(SQUARE).unwrap();
        let reversed = t.arc(!0).unwrap();
        assert_eq!(reversed.first().unwrap().xy(), (10.0, 10.0));
        assert_eq!(reversed.last().unwrap().xy(), (-10.0, -10.0));
    }

    #[test]
    fn missing_objects_are_reported() {
        let t: Topology = serde_json::from_str(SQUARE).unwrap();
        assert!(matches!(t.polygons("oceans"), Err(TopoError::MissingObject(_))));
    }
}
