//! Vertex buffers for the batched meshes, as plain arrays. The renderer copies them into
//! GPU meshes; tests check their invariants without a GPU.

use crate::{
    atlas::{Node, NodeKind, Thread, ThreadKind},
    palette,
    sphere::great_arc,
};
use aph_data::graph::EdgeKind;

pub const ARC_SEGMENTS: usize = 64;
pub const ARC_LIFT: f32 = 0.35;
/// Dots sit just above the globe so they never z-fight with it.
pub const DOT_LIFT: f32 = 1.004;

/// Screen-space ribbons: every centerline sample becomes two vertices (side -1 and +1)
/// that the vertex shader pushes apart by half the line width in pixels.
#[derive(Debug, Default, Clone)]
pub struct Ribbons {
    pub position: Vec<[f32; 3]>,
    pub prev: Vec<[f32; 3]>,
    pub next: Vec<[f32; 3]>,
    /// The arc's last point (where it is answered), for arrowheads and dashes measured in pixels.
    pub end: Vec<[f32; 3]>,
    /// (t along the arc 0..1, side -1|+1, year it appears, width in px)|+1, year it appears, width in px)
    pub line: Vec<[f32; 4]>,
    /// Linear RGB, and in `a` the evidence kind (index into [`EVIDENCE`] / 4).
    pub ink: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

/// Shader code for @mentions (past every evidence kind, so dashed).
pub const MENTION_CODE: f32 = 5.0 / 4.0;

/// Evidence kinds in shader order; the last two are drawn dashed.
pub const EVIDENCE: [EdgeKind; 5] = [EdgeKind::Quotes, EdgeKind::Responds, EdgeKind::Cites, EdgeKind::Alludes, EdgeKind::Influenced];

/// Stronger evidence draws a heavier line.
#[must_use]
pub fn line_width(kind: EdgeKind) -> f32 {
    match kind {
        EdgeKind::Quotes => 3.2,
        EdgeKind::Responds => 2.8,
        EdgeKind::Cites => 2.2,
        EdgeKind::Alludes => 1.8,
        EdgeKind::Influenced => 1.6,
    }
}

impl Ribbons {
    #[must_use]
    pub fn build<'a>(threads: impl IntoIterator<Item = &'a Thread>) -> Self {
        let mut r = Self::default();
        for t in threads {
            r.push(t);
        }
        r
    }

    pub fn push(&mut self, thread: &Thread) {
        let points = great_arc(thread.from, thread.to, ARC_SEGMENTS, ARC_LIFT);
        let base = u32::try_from(self.position.len()).expect("ribbons fit u32 indices");
        let (colour, kind, width) = match thread.kind {
            ThreadKind::Response { kind, valence, .. } => {
                let code = EVIDENCE.iter().position(|k| *k == kind).expect("every kind listed") as f32 / 4.0;
                (palette::valence(valence), code, line_width(kind))
            }
            ThreadKind::Mention { .. } => (palette::MENTION, MENTION_CODE, 1.6),
        };
        let [r, g, b] = colour.linear();
        let last = points.len() - 1;
        let end = points[last];
        for (i, &p) in points.iter().enumerate() {
            let (prev, next) = (points[i.saturating_sub(1)], points[(i + 1).min(last)]);
            let t = i as f32 / last as f32;
            for side in [-1.0, 1.0] {
                self.position.push(p.into());
                self.prev.push(prev.into());
                self.next.push(next.into());
                self.end.push(end.into());
                self.line.push([t, side, thread.year as f32, width]);
                self.ink.push([r, g, b, kind]);
            }
        }
        for i in 0..u32::try_from(last).expect("segments fit u32") {
            let (a, b, c, d) = (base + 2 * i, base + 2 * i + 1, base + 2 * i + 2, base + 2 * i + 3);
            self.indices.extend([a, b, c, b, d, c]);
        }
    }

    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.position.len()
    }
}

/// Screen-space billboards: four corners per node around one centre.
#[derive(Debug, Default, Clone)]
pub struct Dots {
    pub center: Vec<[f32; 3]>,
    pub corner: Vec<[f32; 2]>,
    /// (from year, until year, 1 for an idea else 0, radius in px)
    pub params: Vec<[f32; 4]>,
    /// Linear RGBA fill.
    pub fill: Vec<[f32; 4]>,
    /// (religion bit index, or -1 for none; node index) for masking and highlighting.
    pub ids: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

const CORNERS: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];

impl Dots {
    #[must_use]
    pub fn build<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> Self {
        let mut d = Self::default();
        for (i, n) in nodes.into_iter().enumerate() {
            d.push(i, n);
        }
        d
    }

    pub fn push(&mut self, index: usize, node: &Node) {
        let base = u32::try_from(self.center.len()).expect("dots fit u32 indices");
        let centre = (node.at.to_unit() * DOT_LIFT).into();
        let (idea, radius, colour) = match node.kind {
            NodeKind::Idea => (1.0, 7.0, palette::GOLD),
            NodeKind::Aphorism => (0.0, 5.5, node.religion.map_or(palette::INK, palette::religion)),
        };
        let [r, g, b] = colour.linear();
        for corner in CORNERS {
            self.center.push(centre);
            self.corner.push(corner);
            self.params.push([node.moment as f32, node.until as f32, idea, radius]);
            self.fill.push([r, g, b, 1.0]);
            self.ids.push([node.religion.map_or(-1.0, |f| f.index() as f32), index as f32]);
        }
        self.indices.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{atlas::NodeKind, sphere::LatLon};
    use aph_data::graph::Valence;

    fn thread() -> Thread {
        Thread {
            kind: ThreadKind::Response { edge: 0, kind: EdgeKind::Responds, valence: Valence::Mixed },
            from: LatLon::new(48.857, 2.352),
            to: LatLon::new(55.953, -3.188),
            year: 1783,
        }
    }

    #[test]
    fn ribbon_has_two_vertices_per_sample_and_two_triangles_per_segment() {
        let r = Ribbons::build(&[thread(), thread()]);
        assert_eq!(r.vertex_count(), 2 * 2 * (ARC_SEGMENTS + 1));
        assert_eq!(r.indices.len(), 2 * 6 * ARC_SEGMENTS);
        let n = u32::try_from(r.vertex_count()).unwrap();
        assert!(r.indices.iter().all(|&i| i < n));
    }

    #[test]
    fn ribbon_sides_pair_up_and_t_runs_start_to_end() {
        let r = Ribbons::build(&[thread()]);
        for pair in r.line.chunks(2) {
            assert_eq!((pair[0][1], pair[1][1]), (-1.0, 1.0));
            assert_eq!(pair[0][0], pair[1][0]);
        }
        assert_eq!(r.line.first().unwrap()[0], 0.0);
        assert_eq!(r.line.last().unwrap()[0], 1.0);
        assert!(r.line.windows(2).all(|w| w[1][0] >= w[0][0]));
    }

    #[test]
    fn ribbon_endpoints_sit_on_the_globe_and_the_middle_rises() {
        let r = Ribbons::build(&[thread()]);
        let len = |p: [f32; 3]| glam::Vec3::from(p).length();
        assert!((len(r.position[0]) - 1.0).abs() < 1e-5);
        assert!((len(*r.position.last().unwrap()) - 1.0).abs() < 1e-5);
        assert!(len(r.position[ARC_SEGMENTS]) > 1.0);
    }

    #[test]
    fn ribbon_neighbours_clamp_at_the_ends() {
        let r = Ribbons::build(&[thread()]);
        assert_eq!(r.prev[0], r.position[0]);
        assert_eq!(*r.next.last().unwrap(), *r.position.last().unwrap());
    }

    #[test]
    fn dots_are_quads_lifted_off_the_surface() {
        let node = Node { source: crate::atlas::Source::Entry(0), kind: NodeKind::Idea, at: LatLon::new(0.0, 0.0), from: 1900, until: 1931, span: (1900, 1931), moment: 1900.0, label: String::new(), religion: None };
        let d = Dots::build(&[node]);
        assert_eq!(d.center.len(), 4);
        assert_eq!(d.indices, vec![0, 1, 2, 1, 3, 2]);
        assert!((glam::Vec3::from(d.center[0]).length() - DOT_LIFT).abs() < 1e-5);
        assert_eq!(d.params[0], [1900.0, 1931.0, 1.0, 7.0]);
    }
}

#[cfg(test)]
mod evidence_tests {
    use super::*;

    #[test]
    fn every_vertex_knows_its_arc_end_and_kind() {
        let t = Thread {
            kind: ThreadKind::Response { edge: 0, kind: EdgeKind::Influenced, valence: aph_data::graph::Valence::Adopts },
            from: crate::sphere::LatLon::new(0.0, 0.0),
            to: crate::sphere::LatLon::new(0.0, 30.0),
            year: 0,
        };
        let r = Ribbons::build(&[t]);
        assert!(r.end.iter().all(|e| *e == *r.position.last().unwrap()));
        assert!(r.ink.iter().all(|c| c[3] == 1.0), "influenced is the last evidence kind");
        assert_eq!(r.end.len(), r.position.len());
    }
}
