//! The JavaScript API. The page fetches the three data files, then:
//!
//! ```js
//! const page = JSON.parse(wasm.page_model(aphorisms, graph, civs, portraits)); // feed, cards, colours
//! wasm.on_event(json => …);                  // {type: "picked" | "pickedPlace" | "hovered", …}
//! wasm.start("#globe", aphorisms, graph, civs, portraits, land);  // land: cached data/land-50m.json
//! wasm.send(JSON.stringify({type: "setTime", year: 117, window: 0}));
//! ```
//!
//! Commands and events are `aph_globe::host::{HostCommand, HostEvent}` as JSON.

use std::{cell::RefCell, sync::Mutex};

use aph_data::store::Dataset;
use aph_globe::{
    GlobePlugins,
    host::{HostCommand, HostEvent},
};
use aph_scene::{atlas::Atlas, export::PageModel};
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

/// Commands from the page, waiting for the next frame.
static INBOX: Mutex<Vec<HostCommand>> = Mutex::new(Vec::new());

thread_local! {
    /// The page's event callback.
    static LISTENER: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
}

fn load(aphorisms: &str, graph: &str, civilizations: &str, portraits: Option<String>, land: Option<String>) -> Result<Dataset, JsError> {
    Dataset::from_json(aphorisms, graph, civilizations, portraits.as_deref(), land.as_deref()).map_err(|e| JsError::new(&format!("{e:#}")))
}

/// Everything the page's widgets need, as JSON. Node `i` here is dot `i` on the globe.
#[wasm_bindgen]
pub fn page_model(aphorisms: &str, graph: &str, civilizations: &str, portraits: Option<String>) -> Result<String, JsError> {
    let data = load(aphorisms, graph, civilizations, portraits, None)?;
    let atlas = Atlas::build(&data);
    Ok(serde_json::to_string(&PageModel::build(&data, &atlas))?)
}

/// Register the callback that receives globe events as JSON strings.
#[wasm_bindgen]
pub fn on_event(callback: js_sys::Function) {
    LISTENER.with(|l| *l.borrow_mut() = Some(callback));
}

/// Queue a command (a JSON `HostCommand`) for the next frame.
#[wasm_bindgen]
pub fn send(command: &str) -> Result<(), JsError> {
    let command: HostCommand = serde_json::from_str(command)?;
    INBOX.lock().expect("not poisoned").push(command);
    Ok(())
}

/// Start rendering into the canvas matching `selector`. Returns once the frame loop is scheduled.
/// `coasts`: the baked `data/baked/coasts.png`; without it the coastline field is computed here,
/// which takes a while (pass `land` instead, or neither for a globe without coastlines).
#[wasm_bindgen]
pub fn start_globe(
    selector: &str,
    aphorisms: &str,
    graph: &str,
    civilizations: &str,
    portraits: Option<String>,
    land: Option<String>,
    coasts: Option<Vec<u8>>,
) -> Result<(), JsError> {
    console_error_panic_hook::set_once();
    let atlas = Atlas::build(&load(aphorisms, graph, civilizations, portraits, land)?);
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                canvas: Some(selector.to_owned()),
                fit_canvas_to_parent: true,
                // Let the page keep its own keyboard and scroll handling.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GlobePlugins::new(atlas, coasts))
        .add_systems(First, deliver_commands)
        .add_systems(Last, forward_events)
        .run();
    Ok(())
}

fn deliver_commands(mut commands: MessageWriter<HostCommand>) {
    commands.write_batch(INBOX.lock().expect("not poisoned").drain(..));
}

fn forward_events(mut events: MessageReader<HostEvent>) {
    LISTENER.with(|l| {
        let Some(callback) = &*l.borrow() else { return };
        for event in events.read() {
            if let Ok(json) = serde_json::to_string(event) {
                // A throwing listener is the page's bug; keep rendering.
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&json));
            }
        }
    });
}
