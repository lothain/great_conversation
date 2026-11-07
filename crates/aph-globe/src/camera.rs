//! Orbit camera: drag (mouse or one finger) to turn the globe, scroll or pinch to zoom, and
//! animated focus from the host.
//! The camera always looks at the centre from above a latitude/longitude, so north stays up.

use aph_scene::sphere::LatLon;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
};

/// Where the camera hovers, and how far from the globe's centre (radius 1).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    pub over: LatLon,
    pub distance: f32,
}

impl Orbit {
    pub const MIN_DISTANCE: f32 = 1.25;
    pub const MAX_DISTANCE: f32 = 6.0;

    #[must_use]
    pub fn transform(self) -> Transform {
        Transform::from_translation(self.over.to_unit() * self.distance).looking_at(Vec3::ZERO, Vec3::Y)
    }

    /// Turn by a pointer drag in pixels; slower when zoomed in.
    pub fn drag(&mut self, delta: Vec2) {
        let speed = 0.25 * f64::from(self.distance - 1.0);
        self.over.lon -= f64::from(delta.x) * speed;
        self.over.lat = (self.over.lat + f64::from(delta.y) * speed).clamp(-85.0, 85.0);
        self.over.lon = (self.over.lon + 540.0).rem_euclid(360.0) - 180.0;
    }

    /// Zoom by scroll lines (positive = in).
    pub fn zoom(&mut self, lines: f32) {
        let above = (self.distance - 1.0) * (1.0 - 0.1 * lines).clamp(0.5, 1.5);
        self.distance = (1.0 + above).clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
    }

    /// Pinch: the fingers moved apart by `spread` (new / old distance between them), so the
    /// surface should look that much bigger.
    pub fn pinch(&mut self, spread: f32) {
        if spread.is_finite() && spread > 0.0 {
            self.distance = (1.0 + (self.distance - 1.0) / spread).clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
        }
    }

    /// Part-way from `self` to `to` along the great circle, `t` in 0..=1.
    #[must_use]
    pub fn toward(self, to: Self, t: f32) -> Self {
        let (a, b) = (self.over.to_unit(), to.over.to_unit());
        let dir = if a.dot(b) > 0.9999 { b } else { Quat::from_rotation_arc(a, b).slerp(Quat::IDENTITY, 1.0 - t) * a };
        let mut over = LatLon::from_unit(dir.normalize());
        over.lat = over.lat.clamp(-85.0, 85.0);
        Self { over, distance: self.distance + (to.distance - self.distance) * t }
    }
}

/// A requested camera target; the camera eases toward it, and dragging cancels it.
#[derive(Resource, Debug, Default)]
pub struct Focus(pub Option<Orbit>);

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Focus>().add_systems(Startup, spawn).add_systems(Update, (control, ease).chain());
    }
}

fn spawn(mut commands: Commands) {
    let orbit = Orbit { over: LatLon::new(38.0, 25.0), distance: 3.1 };
    commands.spawn((Camera3d::default(), orbit, orbit.transform()));
}

fn control(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut focus: ResMut<Focus>,
    mut orbit: Single<&mut Orbit>,
) {
    if buttons.pressed(MouseButton::Left) && motion.delta != Vec2::ZERO {
        orbit.drag(motion.delta);
        focus.0 = None;
    }
    if scroll.delta.y != 0.0 {
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        orbit.zoom(lines);
    }
    // Touch: one finger turns the globe, two pinch to zoom (and turn by their midpoint).
    let fingers: Vec<_> = touches.iter().collect();
    match fingers.as_slice() {
        [one] if one.delta() != Vec2::ZERO => {
            orbit.drag(one.delta());
            focus.0 = None;
        }
        [a, b] => {
            let (now, before) = (a.position().distance(b.position()), a.previous_position().distance(b.previous_position()));
            if before > 1.0 {
                orbit.pinch(now / before);
            }
            orbit.drag((a.delta() + b.delta()) / 2.0);
            focus.0 = None;
        }
        _ => {}
    }
}

fn ease(time: Res<Time>, mut focus: ResMut<Focus>, camera: Single<(&mut Orbit, &mut Transform)>) {
    let (mut orbit, mut transform) = camera.into_inner();
    if let Some(target) = focus.0 {
        // Exponential ease: covers ~95% of the way in about a second.
        let t = 1.0 - (-3.0 * time.delta_secs()).exp();
        *orbit = orbit.toward(target, t);
        if orbit.over.to_unit().distance(target.over.to_unit()) < 1e-4 && (orbit.distance - target.distance).abs() < 1e-3 {
            *orbit = target;
            focus.0 = None;
        }
    }
    if orbit.is_changed() {
        *transform = orbit.transform();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_looks_at_the_point_it_hovers_over() {
        let orbit = Orbit { over: LatLon::new(41.9, 12.5), distance: 3.0 };
        let t = orbit.transform();
        let hit = aph_scene::pick::ray_globe(t.translation, *t.forward()).unwrap();
        assert!((hit.lat - 41.9).abs() < 1e-3 && (hit.lon - 12.5).abs() < 1e-3);
    }

    #[test]
    fn pinching_apart_zooms_in_and_is_clamped() {
        let mut orbit = Orbit { over: LatLon::new(0.0, 0.0), distance: 3.0 };
        orbit.pinch(2.0);
        assert!((orbit.distance - 2.0).abs() < 1e-6);
        orbit.pinch(100.0);
        assert!((orbit.distance - Orbit::MIN_DISTANCE).abs() < 1e-6);
        orbit.pinch(0.0);
        assert!((orbit.distance - Orbit::MIN_DISTANCE).abs() < 1e-6, "a degenerate pinch changes nothing");
    }

    #[test]
    fn zoom_is_clamped() {
        let mut orbit = Orbit { over: LatLon::new(0.0, 0.0), distance: 3.0 };
        for _ in 0..100 {
            orbit.zoom(5.0);
        }
        assert!((orbit.distance - Orbit::MIN_DISTANCE).abs() < 1e-6);
        for _ in 0..100 {
            orbit.zoom(-5.0);
        }
        assert!((orbit.distance - Orbit::MAX_DISTANCE).abs() < 1e-6);
    }

    #[test]
    fn dragging_wraps_longitude_and_clamps_latitude() {
        let mut orbit = Orbit { over: LatLon::new(80.0, 175.0), distance: 2.0 };
        orbit.drag(Vec2::new(-100.0, 400.0));
        assert!((-180.0..180.0).contains(&orbit.over.lon));
        assert!(orbit.over.lat <= 85.0);
    }

    #[test]
    fn easing_reaches_the_target_along_the_great_circle() {
        let a = Orbit { over: LatLon::new(0.0, 0.0), distance: 3.0 };
        let b = Orbit { over: LatLon::new(0.0, 90.0), distance: 2.0 };
        let mid = a.toward(b, 0.5);
        assert!((mid.over.lon - 45.0).abs() < 1e-3 && mid.over.lat.abs() < 1e-3);
        assert!((mid.distance - 2.5).abs() < 1e-6);
        assert_eq!(a.toward(b, 1.0).distance, 2.0);
    }
}
