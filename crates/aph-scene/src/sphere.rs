//! Latitude/longitude on the unit globe. Y is north; longitude 0 faces +Z.

use glam::Vec3;

/// A point on the globe in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    #[must_use]
    pub fn new(lat: impl Into<f64>, lon: impl Into<f64>) -> Self {
        Self { lat: lat.into(), lon: lon.into() }
    }

    /// Unit vector for this point.
    #[must_use]
    pub fn to_unit(self) -> Vec3 {
        let (lat, lon) = (self.lat.to_radians(), self.lon.to_radians());
        Vec3::new((lat.cos() * lon.sin()) as f32, lat.sin() as f32, (lat.cos() * lon.cos()) as f32)
    }

    /// Mean position of several points (fine for points that are near each other).
    #[must_use]
    pub fn centroid(points: &[Self]) -> Option<Self> {
        let sum = points.iter().map(|p| p.to_unit()).sum::<Vec3>();
        (!points.is_empty() && sum.length_squared() > 1e-9).then(|| Self::from_unit(sum.normalize()))
    }

    #[must_use]
    pub fn from_unit(v: Vec3) -> Self {
        Self { lat: f64::from(v.y.clamp(-1.0, 1.0).asin().to_degrees()), lon: f64::from(v.x.atan2(v.z).to_degrees()) }
    }
}

/// Points along the great circle from `a` to `b`, lifted into an arc whose height grows with distance.
#[must_use]
pub fn great_arc(a: LatLon, b: LatLon, segments: usize, lift: f32) -> Vec<Vec3> {
    let (pa, pb) = (a.to_unit(), b.to_unit());
    let angle = pa.angle_between(pb);
    let height = lift * angle / std::f32::consts::PI;
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let dir = slerp(pa, pb, angle, t);
            dir * (1.0 + height * (std::f32::consts::PI * t).sin())
        })
        .collect()
}

fn slerp(a: Vec3, b: Vec3, angle: f32, t: f32) -> Vec3 {
    if angle < 1e-4 {
        return a.lerp(b, t).normalize();
    }
    let s = angle.sin();
    (a * ((1.0 - t) * angle).sin() / s + b * (t * angle).sin() / s).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_round_trip() {
        let p = LatLon::new(38.0, 25.0);
        let q = LatLon::from_unit(p.to_unit());
        assert!((p.lat - q.lat).abs() < 1e-4 && (p.lon - q.lon).abs() < 1e-4);
    }

    #[test]
    fn arc_ends_on_its_endpoints() {
        let (a, b) = (LatLon::new(51.5, -0.1), LatLon::new(40.7, -74.0));
        let arc = great_arc(a, b, 16, 0.35);
        assert!(arc[0].distance(a.to_unit()) < 1e-5);
        assert!(arc[16].distance(b.to_unit()) < 1e-5);
        assert!(arc[8].length() > 1.0);
    }
}
