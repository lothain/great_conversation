//! Equirectangular texture helpers shared by the globe's land layers: which civilization
//! snapshots are in effect at a year, the coastline distance field, and a scanline polygon fill
//! (used by [`crate::regions`]).

use aph_data::civ::Polygon;

use crate::atlas::{CivSnapshot, Year};

/// Distances are stored up to this many degrees from an edge.
pub const BORDER_RANGE_DEG: f64 = 1.5;

#[derive(Debug, Clone)]
pub struct Overlay {
    pub width: u32,
    pub height: u32,
    /// sRGB RGBA8, row 0 at latitude +90, column 0 at longitude -180.
    pub rgba: Vec<u8>,
}

impl Overlay {
    #[must_use]
    pub fn empty(width: u32, height: u32) -> Self {
        Self { width, height, rgba: vec![0; (width * height * 4) as usize] }
    }

    /// Centre of texel (x, y) as (lon, lat).
    #[must_use]
    pub fn texel_lonlat(&self, x: u32, y: u32) -> (f64, f64) {
        let lon = (f64::from(x) + 0.5) / f64::from(self.width) * 360.0 - 180.0;
        let lat = 90.0 - (f64::from(y) + 0.5) / f64::from(self.height) * 180.0;
        (lon, lat)
    }

    #[must_use]
    pub fn texel_at(&self, lon: f64, lat: f64) -> (u32, u32) {
        let x = ((lon + 180.0) / 360.0 * f64::from(self.width)).floor().clamp(0.0, f64::from(self.width - 1));
        let y = ((90.0 - lat) / 180.0 * f64::from(self.height)).floor().clamp(0.0, f64::from(self.height - 1));
        (x as u32, y as u32)
    }

    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

}

/// Indices of the snapshots in effect at `year`: per civilization, the latest one whose
/// validity interval contains the year.
#[must_use]
pub fn active(civs: &[CivSnapshot], year: Year) -> Vec<usize> {
    let mut chosen: Vec<usize> = Vec::new();
    for (i, c) in civs.iter().enumerate().filter(|(_, c)| (c.from..=c.until).contains(&year)) {
        match chosen.iter_mut().find(|j| civs[**j].civ == c.civ) {
            Some(j) if civs[*j].from < c.from => *j = i,
            Some(_) => {}
            None => chosen.push(i),
        }
    }
    chosen
}

/// Coastlines: a signed distance field, per texel (same layout as [`Overlay`]), of the distance in
/// degrees to the nearest coast, negative on land, encoded as `128 + 127 * d / BORDER_RANGE_DEG`.
///
/// Signed matters: bilinear filtering interpolates a sign change exactly, so the zero line
/// (the coast) is continuous between texels; an unsigned field would break into dots. The
/// shader turns degrees into pixels with `fwidth`, for constant-width lines at any zoom.
#[must_use]
pub fn coasts(land: &[Polygon], width: u32, height: u32) -> Vec<u8> {
    let polygons: Vec<&Polygon> = land.iter().collect();
    signed_field(&polygons, BORDER_RANGE_DEG, width, height)
}

/// Signed distance to the nearest edge of `polygons` (negative inside any of them), encoded as
/// `128 + 127 * d / range` and clamped. See [`borders`] for why signed.
fn signed_field(polygons: &[&Polygon], range: f64, width: u32, height: u32) -> Vec<u8> {
    let grid = Overlay { width, height, rgba: Vec::new() };
    let mut best = vec![f64::INFINITY; (width * height) as usize];
    let rings = polygons.iter().flat_map(|p| p.iter()).map(|ring| unwrapped(ring));
    for ring in rings {
        for w in ring.windows(2) {
            let ((x1, y1), (x2, y2)) = (w[0], w[1]);
            // Only texels within range of this edge can be affected: visit its expanded bounding box.
            let (lat_lo, lat_hi) = (y1.min(y2) - range, y1.max(y2) + range);
            let k_min = lat_lo.abs().max(lat_hi.abs()).min(89.0).to_radians().cos().max(0.05);
            let lon_pad = range / k_min;
            for shift in COPIES {
                let (a, b) = (x1 + shift, x2 + shift);
                if a.max(b) + lon_pad < -180.0 || a.min(b) - lon_pad > 180.0 {
                    continue;
                }
                let (gx0, gy0) = grid.texel_at(a.min(b) - lon_pad, lat_hi);
                let (gx1, gy1) = grid.texel_at(a.max(b) + lon_pad, lat_lo);
                for y in gy0..=gy1 {
                    for x in gx0..=gx1 {
                        let (px, py) = grid.texel_lonlat(x, y);
                        let k = py.to_radians().cos().max(0.05);
                        let d = segment_distance((px * k, py), (a * k, y1), (b * k, y2));
                        let cell = &mut best[(y * width + x) as usize];
                        *cell = cell.min(d);
                    }
                }
            }
        }
    }
    let interior_bias = 22.5 / f64::from(height); // an eighth of a texel
    // Texels beyond range of every edge still need a sign: deep inside or far outside.
    let mut interior = vec![false; (width * height) as usize];
    for polygon in polygons {
        fill_scanlines(polygon, &grid, |x, y| interior[(y * width + x) as usize] = true);
    }
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .zip(best)
        .map(|((x, y), d)| {
            if d.is_infinite() {
                return if interior[(y * width + x) as usize] { 1 } else { 255 };
            }
            // Sign by "inside any painted core", not by the edge's owner: where cores overlap the
            // owner flips between edges, and an owner-relative sign would invent borders there.
            // Inside, push values an eighth of a texel further from zero. Edges *within* the painted area
            // (one civilization inside another) then never come close enough to zero to speckle
            // when magnified; the outer border moves inward by an invisible fraction of a texel.
            let signed = if interior[(y * width + x) as usize] { -(d + interior_bias) } else { d };
            (128.0 + 127.0 * (signed / range).clamp(-1.0, 1.0)).round() as u8
        })
        .collect()
}

/// Call `f` for every texel whose centre is inside `polygon` (even-odd across all its rings, so
/// holes are excluded): per row, intersect the edges with the row's latitude and fill between
/// pairs of crossings. O(rows × edges) instead of O(texels × edges).
pub(crate) fn fill_scanlines(polygon: &Polygon, grid: &Overlay, mut f: impl FnMut(u32, u32)) {
    let Some((min, max)) = bounds(std::slice::from_ref(polygon)) else { return };
    let (_, y0) = grid.texel_at(min.0, max.1);
    let (_, y1) = grid.texel_at(max.0, min.1);
    let rings: Vec<Vec<(f64, f64)>> = polygon.iter().map(|r| unwrapped(r)).collect();
    let mut crossings = Vec::new();
    for y in y0..=y1 {
        let (_, lat) = grid.texel_lonlat(0, y);
        crossings.clear();
        for w in rings.iter().flat_map(|ring| ring.windows(2)) {
            let ((x1, y1), (x2, y2)) = (w[0], w[1]);
            if (y1 > lat) != (y2 > lat) {
                crossings.push(x1 + (lat - y1) / (y2 - y1) * (x2 - x1));
            }
        }
        crossings.sort_by(f64::total_cmp);
        for pair in crossings.chunks_exact(2) {
            for shift in COPIES {
                let (s0, s1) = (pair[0] + shift, pair[1] + shift);
                if s1 < -180.0 || s0 > 180.0 {
                    continue;
                }
                let (from, _) = grid.texel_at(s0, lat);
                let (to, _) = grid.texel_at(s1, lat);
                for x in from..=to {
                    let (lon, _) = grid.texel_lonlat(x, y);
                    if (s0..=s1).contains(&lon) {
                        f(x, y);
                    }
                }
            }
        }
    }
}

/// A ring's points with continuous longitudes: crossing the antimeridian (179.9° then -179.9°)
/// continues to 180.1° instead of jumping back across the globe. Work is done in this unwrapped
/// space and mapped back onto the texture through the ±360° [`COPIES`].
fn unwrapped(ring: &[aph_data::geo::Position]) -> Vec<(f64, f64)> {
    let mut shift = 0.0;
    let mut previous: Option<f64> = None;
    ring.iter()
        .map(|p| {
            let (x, y) = p.xy();
            if let Some(px) = previous {
                if x - px > 180.0 {
                    shift -= 360.0;
                } else if px - x > 180.0 {
                    shift += 360.0;
                }
            }
            previous = Some(x);
            (x + shift, y)
        })
        .collect()
}

/// Longitude offsets under which an unwrapped shape may overlap the map.
const COPIES: [f64; 3] = [0.0, -360.0, 360.0];

fn bounds(polygons: &[Polygon]) -> Option<((f64, f64), (f64, f64))> {
    let pts = polygons.iter().flatten().flatten().map(|p| p.xy());
    pts.fold(None, |acc, (x, y)| {
        let ((ax, ay), (bx, by)) = acc.unwrap_or(((x, y), (x, y)));
        Some(((ax.min(x), ay.min(y)), (bx.max(x), by.max(y))))
    })
}

fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aph_data::{geo::Position, ids::CivId};

    fn square(half: f64) -> Polygon {
        let p = |x: f64, y: f64| Position::try_from([x, y]).unwrap();
        vec![vec![p(-half, -half), p(half, -half), p(half, half), p(-half, half), p(-half, -half)]]
    }

    fn snapshot(civ: &str, from: Year, until: Year) -> CivSnapshot {
        CivSnapshot { civ: CivId(civ.into()), name: civ.into(), from, until, core: vec![square(10.0)], halo: vec![square(20.0)], religions: vec![], note: String::new() }
    }

    #[test]
    fn border_field_is_signed_zero_on_the_edge_negative_inside() {
        let civs = [snapshot("test", 0, 100)];
        let (w, h) = (3600, 1800);
        let field = coasts(&civs[0].core, w, h);
        let grid = Overlay { width: w, height: h, rgba: Vec::new() };
        let at = |lon: f64, lat: f64| { let (x, y) = grid.texel_at(lon, lat); i32::from(field[(y * w + x) as usize]) };
        assert!((at(10.0, 0.0) - 128).abs() <= 4, "on the edge: {}", at(10.0, 0.0));
        assert!((at(10.75, 0.0) - 192).abs() <= 4, "half the range outside: {}", at(10.75, 0.0));
        assert!((at(9.25, 0.0) - 64).abs() <= 4, "half the range inside: {}", at(9.25, 0.0));
        assert_eq!(at(0.0, 0.0), 1, "far inside: clamped");
        assert!((at(10.0, 1.0) - 128).abs() <= 4, "anywhere along the edge");
    }

    #[test]
    fn the_field_crosses_the_border_value_exactly_once_from_centre_to_outside() {
        let civs = [snapshot("test", 0, 100)];
        let (w, h) = (1440, 720);
        let field = coasts(&civs[0].core, w, h);
        let grid = Overlay { width: w, height: h, rgba: Vec::new() };
        let (_, y) = grid.texel_at(0.0, 0.0);
        let row: Vec<i32> = (0..w).map(|x| i32::from(field[(y * w + x) as usize]) - 128).collect();
        let (cx, _) = grid.texel_at(0.0, 0.0);
        let (ox, _) = grid.texel_at(40.0, 0.0);
        let crossings = row[cx as usize..=ox as usize].windows(2).filter(|p| (p[0] < 0) != (p[1] < 0)).count();
        assert_eq!(crossings, 1, "a phantom border would add crossings");
    }

    #[test]
    fn no_active_civs_means_no_borders() {
        assert!(coasts(&[], 360, 180).iter().all(|&v| v == 255));
    }

    #[test]
    fn active_picks_the_latest_snapshot_per_civ() {
        let civs = [snapshot("a", 0, 500), snapshot("a", 300, 800), snapshot("b", 600, 900)];
        assert_eq!(active(&civs, 100), vec![0]);
        assert_eq!(active(&civs, 400), vec![1]);
        assert_eq!(active(&civs, 700), vec![1, 2]);
        assert!(active(&civs, 1000).is_empty());
    }
}

#[cfg(test)]
mod antimeridian_tests {
    use super::*;
    use aph_data::geo::Position;

    #[test]
    fn edges_jumping_the_antimeridian_draw_no_line_and_fill_no_row() {
        let p = |x: f64, y: f64| Position::try_from([x, y]).unwrap();
        // A sliver hugging the antimeridian whose ring jumps from +179 to -179 and back.
        let ring = vec![p(170.0, 60.0), p(179.0, 60.0), p(-179.0, 61.0), p(-170.0, 61.0), p(-170.0, 62.0), p(-179.0, 62.0), p(179.0, 63.0), p(170.0, 63.0), p(170.0, 60.0)];
        let field = coasts(&[vec![ring]], 720, 360);
        let grid = Overlay::empty(720, 360);
        let at = |lon: f64, lat: f64| { let (x, y) = grid.texel_at(lon, lat); field[(y * 720 + x) as usize] };
        assert_eq!(at(0.0, 61.5), 255, "nothing on the far side of the globe at that latitude");
    }
}
