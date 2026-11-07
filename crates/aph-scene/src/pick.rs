//! Turning a pointer ray into a place on the globe and the item under it.

use glam::Vec3;

use crate::{atlas::Node, sphere::LatLon, time::Clock};

/// First intersection of a ray with the unit sphere at the origin.
#[must_use]
pub fn ray_globe(origin: Vec3, dir: Vec3) -> Option<LatLon> {
    let dir = dir.try_normalize()?;
    let b = origin.dot(dir);
    let c = origin.length_squared() - 1.0;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t >= 0.0).then(|| LatLon::from_unit(origin + dir * t))
}

/// Great-circle distance in degrees.
#[must_use]
pub fn angular_distance(a: LatLon, b: LatLon) -> f64 {
    f64::from(a.to_unit().angle_between(b.to_unit()).to_degrees())
}

/// The visible node nearest `at`, within `radius` degrees.
#[must_use]
pub fn nearest(nodes: &[Node], clock: Clock, at: LatLon, radius: f64) -> Option<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| clock.shows(n.moment as f32, n.until as f32))
        .map(|(i, n)| (i, angular_distance(n.at, at)))
        .filter(|&(_, d)| d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::NodeKind;

    fn node(lat: f64, lon: f64, year: i32) -> Node {
        Node { source: crate::atlas::Source::Entry(0), kind: NodeKind::Aphorism, at: LatLon::new(lat, lon), from: year, until: year, span: (year, year), moment: f64::from(year), label: String::new(), religion: None }
    }

    #[test]
    fn ray_from_outside_hits_the_near_side() {
        let hit = ray_globe(Vec3::new(0.0, 0.0, 3.0), Vec3::NEG_Z).unwrap();
        assert!(hit.lat.abs() < 1e-4 && hit.lon.abs() < 1e-4);
    }

    #[test]
    fn ray_that_misses_returns_none() {
        assert!(ray_globe(Vec3::new(0.0, 2.0, 3.0), Vec3::NEG_Z).is_none());
        assert!(ray_globe(Vec3::new(0.0, 0.0, 3.0), Vec3::Z).is_none(), "pointing away");
    }

    #[test]
    fn nearest_ignores_items_not_yet_visible() {
        let nodes = [node(0.0, 0.0, 1900), node(0.5, 0.5, 1700)];
        let here = LatLon::new(0.0, 0.0);
        assert_eq!(nearest(&nodes, Clock::at(2000.0), here, 2.0), Some(0));
        assert_eq!(nearest(&nodes, Clock::at(1800.0), here, 2.0), Some(1));
        assert_eq!(nearest(&nodes, Clock::at(1600.0), here, 2.0), None);
    }

    #[test]
    fn nearest_respects_the_radius() {
        let nodes = [node(10.0, 10.0, 0)];
        assert_eq!(nearest(&nodes, Clock::at(100.0), LatLon::new(0.0, 0.0), 2.0), None);
    }
}
