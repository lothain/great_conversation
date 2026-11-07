//! Pointer → globe. The globe shows no popups of its own; it reports and moves:
//!
//! - **click** a dot: report every dot whose disc is under the pointer (`Picked`), nearest
//!   first. The host decides what that means (one dot: that utterance; several: maybe none).
//! - **double-click** a dot: centre on the dots around it and zoom in until they separate.
//! - **hover**: report whether a dot is under the pointer (for the cursor), nothing more.

use aph_scene::{atlas::NodeKind, buffers::DOT_LIFT, sphere::LatLon};
use bevy::{camera::Projection, prelude::*, window::PrimaryWindow};

use crate::{
    camera::{Focus, Orbit},
    clock::ShownTime,
    host::HostEvent,
    scene::Scene,
};

/// A press that moves less than this many pixels is a click, not a drag.
const CLICK_SLOP: f32 = 5.0;
/// Two clicks this close in time and space are a double-click.
const DOUBLE_CLICK_SECS: f32 = 0.35;
const DOUBLE_CLICK_SLOP: f32 = 8.0;
/// Extra pixels around a dot's disc that still count as on it.
const HIT_MARGIN: f32 = 2.0;
/// Dots within this many pixels of a double-click are pulled apart together.
const DECLUTTER_REACH: f32 = 28.0;
/// How far apart (in pixels) a double-click tries to set the nearest two dots.
const DECLUTTER_GAP: f32 = 18.0;

/// Dot radii in logical pixels; must match `aph_scene::buffers::Dots`.
fn dot_radius(kind: NodeKind) -> f32 {
    match kind {
        NodeKind::Idea => 7.0,
        NodeKind::Aphorism => 5.5,
    }
}

#[derive(Resource, Debug, Default)]
struct Pointer {
    pressed_at: Option<Vec2>,
    last_click: Option<(f32, Vec2)>,
    hovering: bool,
}

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Pointer>().add_systems(Update, (click, hover, report_view));
    }
}

/// A dot on screen: node index, screen position, radius, and distance from the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnScreen {
    pub node: u32,
    pub at: Vec2,
    pub radius: f32,
    pub distance: f32,
}

/// Every visible, front-facing dot within `reach` pixels of the pointer, nearest first.
fn dots_near(scene: &Scene, shown: &ShownTime, camera: (&Camera, &GlobalTransform), cursor: Vec2, reach: f32) -> Vec<OnScreen> {
    let (cam, transform) = camera;
    let eye = transform.translation();
    let mut near: Vec<OnScreen> = scene
        .atlas
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| shown.0.shows(n.from as f32, n.until as f32))
        .filter_map(|(i, n)| {
            let world = n.at.to_unit() * DOT_LIFT;
            // Dots on the far side of the globe can't be clicked.
            if world.dot(eye - world) <= 0.0 {
                return None;
            }
            let at = cam.world_to_viewport(transform, world).ok()?;
            let distance = at.distance(cursor);
            let radius = dot_radius(n.kind);
            (distance <= radius + reach).then_some(OnScreen { node: i as u32, at, radius, distance })
        })
        .collect();
    near.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    near
}

/// The dots whose discs (plus a small margin) contain the pointer.
#[must_use]
pub fn under_pointer(near: &[OnScreen]) -> Vec<u32> {
    near.iter().filter(|d| d.distance <= d.radius + HIT_MARGIN).map(|d| d.node).collect()
}

/// Camera distance (from the globe's centre) at which the closest two of `points` are
/// `gap` pixels apart, given `focal` pixels per radian at unit distance. Never zooms out
/// past `current` or in past `min`. Points at the same place can't be separated and are ignored.
#[must_use]
pub fn declutter_distance(points: &[LatLon], focal: f32, gap: f32, current: f32, min: f32) -> f32 {
    let units: Vec<Vec3> = points.iter().map(|p| p.to_unit()).collect();
    let closest = units
        .iter()
        .enumerate()
        // Chord form: accurate for tiny angles, where acos of a dot product rounds to zero in f32.
        .flat_map(|(i, a)| units[i + 1..].iter().map(move |b| 2.0 * (a.distance(*b) / 2.0).min(1.0).asin()))
        .filter(|&angle| angle > 1e-5)
        .fold(f32::INFINITY, f32::min);
    if closest.is_infinite() {
        // Nothing to separate: just move in a step.
        return (1.0 + (current - 1.0) * 0.6).clamp(min, current);
    }
    // At camera distance d the surface under the camera is d − 1 away, so an angle θ there
    // spans θ · focal / (d − 1) pixels.
    (1.0 + closest * focal / gap).clamp(min, current)
}

fn click(
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform, &Projection, &Orbit)>,
    scene: Res<Scene>,
    shown: Res<ShownTime>,
    mut pointer: ResMut<Pointer>,
    mut focus: ResMut<Focus>,
    mut events: MessageWriter<HostEvent>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        pointer.pressed_at = window.cursor_position();
    }
    // A tap is a click: the last finger lifting where it went down (not the end of a pinch).
    let tap = touches.iter_just_released().next().filter(|_| touches.iter().next().is_none()).map(|t| (t.start_position(), t.position()));
    let (start, cursor) = if buttons.just_released(MouseButton::Left) {
        let (Some(start), Some(cursor)) = (pointer.pressed_at.take(), window.cursor_position()) else { return };
        (start, cursor)
    } else if let Some(tap) = tap {
        tap
    } else {
        return;
    };
    if start.distance(cursor) > CLICK_SLOP {
        return; // a drag, which turned the globe
    }
    let (cam, transform, projection, orbit) = *camera;
    let now = time.elapsed_secs();
    let double = pointer.last_click.is_some_and(|(t, at)| now - t <= DOUBLE_CLICK_SECS && at.distance(cursor) <= DOUBLE_CLICK_SLOP);
    pointer.last_click = if double { None } else { Some((now, cursor)) };

    let near = dots_near(&scene, &shown, (cam, transform), cursor, DECLUTTER_REACH);
    let hits = under_pointer(&near);
    if double && !hits.is_empty() {
        let points: Vec<LatLon> = near.iter().map(|d| scene.atlas.nodes[d.node as usize].at).collect();
        let fov = match projection {
            Projection::Perspective(p) => p.fov,
            _ => std::f32::consts::FRAC_PI_4,
        };
        let focal = window.height() / (2.0 * (fov / 2.0).tan());
        let distance = declutter_distance(&points, focal, DECLUTTER_GAP, orbit.distance, Orbit::MIN_DISTANCE);
        let over = LatLon::centroid(&points).unwrap_or(orbit.over);
        focus.0 = Some(Orbit { over, distance });
    } else if !double {
        // Empty clicks are reported too (as no nodes), so the host can treat them as "clicked away".
        events.write(HostEvent::Picked { nodes: hits });
    }
}

fn hover(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    scene: Res<Scene>,
    shown: Res<ShownTime>,
    mut pointer: ResMut<Pointer>,
    mut events: MessageWriter<HostEvent>,
) {
    let over_dot = window.cursor_position().is_some_and(|cursor| !under_pointer(&dots_near(&scene, &shown, *camera, cursor, 0.0)).is_empty());
    if over_dot != pointer.hovering {
        pointer.hovering = over_dot;
        events.write(HostEvent::Hovered { over_dot });
    }
}

/// Tell the host where the camera is, at most a few times a second while it moves.
fn report_view(time: Res<Time>, orbit: Query<&Orbit, Changed<Orbit>>, mut last: Local<f32>, mut events: MessageWriter<HostEvent>) {
    let Ok(orbit) = orbit.single() else { return };
    let now = time.elapsed_secs();
    if now - *last >= 0.2 {
        *last = now;
        events.write(HostEvent::View { lat: orbit.over.lat, lon: orbit.over.lon, distance: orbit.distance });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(node: u32, distance: f32) -> OnScreen {
        OnScreen { node, at: Vec2::ZERO, radius: 5.5, distance }
    }

    #[test]
    fn only_dots_whose_disc_contains_the_pointer_are_under_it() {
        let near = [dot(3, 1.0), dot(7, 6.0), dot(9, 20.0)];
        assert_eq!(under_pointer(&near), vec![3, 7]);
        assert!(under_pointer(&[dot(1, 9.0)]).is_empty());
    }

    #[test]
    fn decluttering_zooms_until_the_closest_pair_is_the_gap_apart() {
        let points = [LatLon::new(0.0, 0.0), LatLon::new(0.0, 1.0), LatLon::new(0.0, 5.0)];
        let focal = 1000.0;
        let d = declutter_distance(&points, focal, 20.0, 5.0, 1.01);
        let closest = 1.0_f32.to_radians();
        let px = closest * focal / (d - 1.0);
        assert!((px - 20.0).abs() < 1e-3, "closest pair ends up {px} px apart");
    }

    #[test]
    fn decluttering_never_zooms_out_or_past_the_limit() {
        let far_apart = [LatLon::new(0.0, 0.0), LatLon::new(0.0, 60.0)];
        assert_eq!(declutter_distance(&far_apart, 1000.0, 20.0, 2.0, 1.25), 2.0, "already separate: stay");
        let tight = [LatLon::new(0.0, 0.0), LatLon::new(0.0, 0.001)];
        assert_eq!(declutter_distance(&tight, 1000.0, 20.0, 3.0, 1.25), 1.25, "too tight: closest allowed");
    }

    #[test]
    fn coincident_dots_just_zoom_in_a_step() {
        let same = [LatLon::new(10.0, 10.0), LatLon::new(10.0, 10.0)];
        let d = declutter_distance(&same, 1000.0, 20.0, 3.0, 1.25);
        assert!((1.25..3.0).contains(&d));
    }
}
