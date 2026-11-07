//! Regions: at any year, every piece of land belongs to exactly one region, and the regions
//! tile the land like puzzle pieces.
//!
//! 1. Land comes from the coastline field (negative = land).
//! 2. Civilization cores are painted largest first, so a smaller civilization inside a larger
//!    one (France inside the Western bloc) is its own piece.
//! 3. Land no core covers goes to the nearest region within [`CLAIM_STEPS`] texels, by a
//!    breadth-first flood that may cross narrow seas but only ever labels land.
//! 4. Land still unclaimed is [`UNKNOWN`]: a region of its own, not a hole.
//!
//! The result is a label per texel that the globe shader colours, drawing a crisp line wherever
//! neighbouring labels differ.

use std::collections::VecDeque;

use aph_data::civ::Polygon;

use crate::{
    atlas::CivSnapshot,
    overlay::{Overlay, fill_scanlines},
};

/// Label for sea.
pub const SEA: u8 = 0;
/// Label for land no civilization reaches.
pub const UNKNOWN: u8 = 1;
/// Labels from here on are civilizations: `FIRST_CIV + i` is `RegionMap::civs[i]`.
pub const FIRST_CIV: u8 = 2;
/// How far (in texels, 4-connected) unclaimed land is pulled into a neighbouring region.
pub const CLAIM_STEPS: u16 = 48;

#[derive(Debug, Clone)]
pub struct RegionMap {
    pub width: u32,
    pub height: u32,
    /// One label per texel, rows from latitude +90.
    pub labels: Vec<u8>,
    /// Snapshot index (into the atlas `civs`) for each civilization label.
    pub civs: Vec<usize>,
}

impl RegionMap {
    #[must_use]
    pub fn label_at(&self, lon: f64, lat: f64) -> u8 {
        let (x, y) = grid(self.width, self.height).texel_at(lon, lat);
        self.labels[(y * self.width + x) as usize]
    }

    /// The snapshot index a label stands for, if it is a civilization.
    #[must_use]
    pub fn civ_of(&self, label: u8) -> Option<usize> {
        label.checked_sub(FIRST_CIV).and_then(|i| self.civs.get(usize::from(i)).copied())
    }
}

fn grid(width: u32, height: u32) -> Overlay {
    Overlay { width, height, rgba: Vec::new() }
}

/// Land mask from a coastline field (`overlay::coasts` encoding: below 128 is land).
#[must_use]
pub fn land_mask(coasts: &[u8]) -> Vec<bool> {
    coasts.iter().map(|&v| v < 128).collect()
}

/// Rough area of a civilization's core in square degrees (latitude-weighted), for ordering.
fn area(polygons: &[Polygon]) -> f64 {
    polygons
        .iter()
        .filter_map(|p| p.first())
        .map(|ring| {
            let twice: f64 = ring
                .windows(2)
                .map(|w| {
                    let ((x1, y1), (x2, y2)) = (w[0].xy(), w[1].xy());
                    (x1 * y2 - x2 * y1) * f64::midpoint(y1, y2).to_radians().cos()
                })
                .sum();
            twice.abs() / 2.0
        })
        .sum()
}

#[must_use]
pub fn regions(civs: &[CivSnapshot], active: &[usize], land: &[bool], width: u32, height: u32) -> RegionMap {
    let g = grid(width, height);
    let n = (width * height) as usize;
    assert_eq!(land.len(), n, "land mask matches the map size");

    // Largest first, so smaller pieces overwrite and survive.
    let mut order: Vec<usize> = active.to_vec();
    order.sort_by(|&a, &b| area(&civs[b].core).total_cmp(&area(&civs[a].core)));
    order.truncate(usize::from(u8::MAX - FIRST_CIV));

    let mut labels = vec![SEA; n];
    for (i, &civ) in order.iter().enumerate() {
        let label = FIRST_CIV + u8::try_from(i).expect("truncated to fit");
        for polygon in &civs[civ].core {
            fill_scanlines(polygon, &g, |x, y| {
                let t = (y * width + x) as usize;
                if land[t] {
                    labels[t] = label;
                }
            });
        }
    }

    // Flood outward from every claimed land texel, through land and sea alike, giving
    // unclaimed land to whichever region reaches it first.
    let mut steps = vec![u16::MAX; n];
    let mut frontier: VecDeque<usize> = VecDeque::new();
    let mut owner = labels.clone();
    for (t, &l) in labels.iter().enumerate() {
        if l >= FIRST_CIV {
            steps[t] = 0;
            frontier.push_back(t);
        }
    }
    while let Some(t) = frontier.pop_front() {
        if steps[t] >= CLAIM_STEPS {
            continue;
        }
        let (x, y) = ((t as u32) % width, (t as u32) / width);
        let neighbours = [
            Some(y * width + (x + width - 1) % width),
            Some(y * width + (x + 1) % width),
            y.checked_sub(1).map(|y| y * width + x),
            (y + 1 < height).then(|| (y + 1) * width + x),
        ];
        for next in neighbours.into_iter().flatten().map(|v| v as usize) {
            if steps[next] == u16::MAX {
                steps[next] = steps[t] + 1;
                owner[next] = owner[t];
                frontier.push_back(next);
            }
        }
    }
    for t in 0..n {
        labels[t] = match (land[t], labels[t]) {
            (false, _) => SEA,
            (true, l) if l >= FIRST_CIV => l,
            (true, _) if owner[t] >= FIRST_CIV => owner[t],
            (true, _) => UNKNOWN,
        };
    }
    RegionMap { width, height, labels, civs: order }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aph_data::{geo::Position, ids::CivId};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
        let p = |x: f64, y: f64| Position::try_from([x, y]).unwrap();
        vec![vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1), p(x0, y0)]]
    }

    fn civ(id: &str, core: Polygon) -> CivSnapshot {
        CivSnapshot { civ: CivId(id.into()), name: id.into(), from: 0, until: 100, core: vec![core], halo: vec![], religions: vec![], note: String::new() }
    }

    /// Land everywhere in [-60, 60] longitude, sea elsewhere.
    fn continent(w: u32, h: u32) -> Vec<bool> {
        let g = grid(w, h);
        (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| g.texel_lonlat(x, y).0.abs() < 60.0).collect()
    }

    #[test]
    fn every_land_texel_has_a_region_and_the_sea_has_none() {
        let (w, h) = (360, 180);
        let land = continent(w, h);
        let civs = [civ("big", rect(-40.0, -30.0, 40.0, 30.0))];
        let map = regions(&civs, &[0], &land, w, h);
        for (t, &l) in map.labels.iter().enumerate() {
            assert_eq!(land[t], l != SEA, "land is labelled, sea is not");
        }
    }

    #[test]
    fn a_smaller_core_inside_a_larger_one_is_its_own_piece() {
        let (w, h) = (360, 180);
        let land = continent(w, h);
        let civs = [civ("bloc", rect(-40.0, -30.0, 40.0, 30.0)), civ("france", rect(-5.0, -5.0, 5.0, 5.0))];
        let map = regions(&civs, &[1, 0], &land, w, h);
        assert_eq!(map.civ_of(map.label_at(0.0, 0.0)), Some(1));
        assert_eq!(map.civ_of(map.label_at(20.0, 20.0)), Some(0));
    }

    #[test]
    fn nearby_unclaimed_land_joins_a_neighbour_and_distant_land_is_unknown() {
        let (w, h) = (360, 180);
        let land = continent(w, h);
        let civs = [civ("small", rect(-10.0, -10.0, 10.0, 10.0))];
        let map = regions(&civs, &[0], &land, w, h);
        assert_eq!(map.civ_of(map.label_at(15.0, 0.0)), Some(0), "5° away: claimed");
        assert_eq!(map.label_at(55.0, 60.0), UNKNOWN, "far away: its own unknown region");
    }

    #[test]
    fn with_no_civilizations_all_land_is_unknown() {
        let (w, h) = (90, 45);
        let land = continent(w, h);
        let map = regions(&[], &[], &land, w, h);
        assert!(map.labels.iter().zip(&land).all(|(&l, &is_land)| l == if is_land { UNKNOWN } else { SEA }));
    }
}

impl RegionMap {
    /// Pairs of civilization labels that share a border (4-neighbourhood, wrapping east–west).
    #[must_use]
    pub fn neighbours(&self) -> std::collections::BTreeSet<(u8, u8)> {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut pairs = std::collections::BTreeSet::new();
        for y in 0..h {
            for x in 0..w {
                let here = self.labels[y * w + x];
                if here < FIRST_CIV {
                    continue;
                }
                let right = self.labels[y * w + (x + 1) % w];
                let below = if y + 1 < h { self.labels[(y + 1) * w + x] } else { here };
                for other in [right, below] {
                    if other >= FIRST_CIV && other != here {
                        pairs.insert((here.min(other), here.max(other)));
                    }
                }
            }
        }
        pairs
    }

    /// A colour index (0..`colours`) per civilization label, such that neighbouring regions
    /// differ. Each region keeps its `preferred` colour when no neighbour has taken it, so a
    /// civilization's colour stays the same from one era to the next wherever possible.
    #[must_use]
    pub fn colouring(&self, colours: usize, preferred: impl Fn(usize) -> usize) -> Vec<usize> {
        let pairs = self.neighbours();
        let mut chosen: Vec<Option<usize>> = vec![None; self.civs.len()];
        // Labels are in decreasing area order: big regions choose first.
        for i in 0..self.civs.len() {
            let label = FIRST_CIV + u8::try_from(i).expect("labels fit u8");
            let taken: Vec<usize> = pairs
                .iter()
                .filter_map(|&(a, b)| match (a == label, b == label) {
                    (true, _) => Some(b),
                    (_, true) => Some(a),
                    _ => None,
                })
                .filter_map(|other| chosen[usize::from(other - FIRST_CIV)])
                .collect();
            let first = preferred(self.civs[i]) % colours;
            let pick = (0..colours).map(|k| (first + k) % colours).find(|c| !taken.contains(c)).unwrap_or(first);
            chosen[i] = Some(pick);
        }
        chosen.into_iter().map(|c| c.unwrap_or(0)).collect()
    }
}

#[cfg(test)]
mod colouring_tests {
    use super::*;
    use aph_data::{geo::Position, ids::CivId};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
        let p = |x: f64, y: f64| Position::try_from([x, y]).unwrap();
        vec![vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1), p(x0, y0)]]
    }

    #[test]
    fn neighbours_never_share_a_colour_even_when_they_prefer_the_same_one() {
        let (w, h) = (360, 180);
        let land = vec![true; (w * h) as usize];
        let civ = |id: &str, core| CivSnapshot { civ: CivId(id.into()), name: id.into(), from: 0, until: 1, core: vec![core], halo: vec![], religions: vec![], note: String::new() };
        let civs = [civ("west", rect(-40.0, -20.0, 0.0, 20.0)), civ("east", rect(0.0, -20.0, 40.0, 20.0)), civ("inner", rect(-10.0, -5.0, -2.0, 5.0))];
        let map = regions(&civs, &[0, 1, 2], &land, w, h);
        let colours = map.colouring(8, |_| 3);
        for (a, b) in map.neighbours() {
            assert_ne!(colours[usize::from(a - FIRST_CIV)], colours[usize::from(b - FIRST_CIV)], "{a} and {b} touch");
        }
        assert!(colours.contains(&3), "someone keeps the preferred colour");
    }
}
