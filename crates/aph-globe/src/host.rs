//! The renderer's API to whoever embeds it: commands in, events out.

use std::collections::BTreeSet;

use aph_scene::{sphere::LatLon, time::Clock};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    camera::{Focus, Orbit},
    clock::ShownTime,
    dots::DotStyle,
    overlay::CivFilter,
};

/// What the host can ask of the globe.
#[derive(Message, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostCommand {
    /// Show this year; `window` years of history (0 = everything so far).
    SetTime { year: f32, window: f32 },
    /// Turn the camera to look at a place; keeps the current zoom when `distance` is absent.
    Focus { lat: f64, lon: f64, distance: Option<f32> },
    /// Only these religions (bit i = `ReligionFamily::ALL[i]`) at full strength; `None` = no mask.
    ReligionMask { bits: Option<u32> },
    /// Only these civilizations are painted; `None` = all.
    CivMask { civs: Option<BTreeSet<String>> },
    /// Ring one node (by index, as in the page model); `None` clears it.
    Highlight { node: Option<u32> },
}

/// What the globe tells the host.
#[derive(Message, Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostEvent {
    /// The globe is drawing (pipelines compiled, textures uploaded): the host can drop its loading state.
    Ready,
    /// A click (not a drag): every dot whose disc was under the pointer, nearest first (by index,
    /// as in the page model). Empty means the globe itself was clicked; one means that utterance;
    /// several overlapping may be ambiguous.
    Picked { nodes: Vec<u32> },
    /// The pointer moved onto a dot or off all dots (for the cursor; the globe shows no popups).
    Hovered { over_dot: bool },
    /// The camera moved (throttled): where it hovers and how far from the centre.
    View { lat: f64, lon: f64, distance: f32 },
}

pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HostCommand>().add_message::<HostEvent>().add_systems(PreUpdate, apply).add_systems(Last, announce_ready);
    }
}

fn apply(
    mut commands: MessageReader<HostCommand>,
    mut shown: ResMut<ShownTime>,
    mut style: ResMut<DotStyle>,
    mut civs: ResMut<CivFilter>,
    mut focus: ResMut<Focus>,
    orbit: Query<&Orbit>,
) {
    for command in commands.read() {
        match command.clone() {
            HostCommand::SetTime { year, window } => shown.0 = Clock { year, window, ..shown.0 },
            HostCommand::Focus { lat, lon, distance } => {
                let current = orbit.single().map_or(3.0, |o| o.distance);
                focus.0 = Some(Orbit { over: LatLon::new(lat, lon), distance: distance.unwrap_or(current) });
            }
            HostCommand::ReligionMask { bits } => style.religion_mask = bits,
            HostCommand::CivMask { civs: set } => civs.0 = set,
            HostCommand::Highlight { node } => style.highlight = node,
        }
    }
}


/// Frames to draw before announcing readiness. Shader compilation and texture uploads happen
/// inside the first frames (synchronously on WebGL), so a few completed frames means the globe
/// is really on screen.
const READY_AFTER_FRAMES: u32 = 4;

fn announce_ready(mut frames: Local<u32>, mut events: MessageWriter<HostEvent>) {
    if *frames <= READY_AFTER_FRAMES {
        *frames += 1;
        if *frames == READY_AFTER_FRAMES {
            events.write(HostEvent::Ready);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_use_the_web_wire_format() {
        let c: HostCommand = serde_json::from_str(r#"{"type":"setTime","year":117,"window":0}"#).unwrap();
        assert_eq!(c, HostCommand::SetTime { year: 117.0, window: 0.0 });
        let c: HostCommand = serde_json::from_str(r#"{"type":"focus","lat":41.9,"lon":12.5}"#).unwrap();
        assert_eq!(c, HostCommand::Focus { lat: 41.9, lon: 12.5, distance: None });
        let c: HostCommand = serde_json::from_str(r#"{"type":"civMask","civs":["roman"]}"#).unwrap();
        assert!(matches!(c, HostCommand::CivMask { civs: Some(s) } if s.contains("roman")));
    }

    #[test]
    fn events_serialize_for_javascript() {
        let e = serde_json::to_string(&HostEvent::Picked { nodes: vec![4, 9] }).unwrap();
        assert_eq!(e, r#"{"type":"picked","nodes":[4,9]}"#);
        let e = serde_json::to_string(&HostEvent::Hovered { over_dot: true }).unwrap();
        assert_eq!(e, r#"{"type":"hovered","overDot":true}"#);
    }
}
