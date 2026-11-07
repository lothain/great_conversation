//! Every data file must survive parse → serialize without losing or changing anything.
//! The one intended normalization: entries always carry `date_note` (null when absent).

use std::path::{Path, PathBuf};

use aph_data::{civ::Civilizations, collection::Collection, graph::Graph, store};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

fn data(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data").join(file)
}

fn assert_lossless<T: DeserializeOwned + Serialize>(file: &str, normalize: impl Fn(&mut Value)) {
    let mut raw: Value = store::read(&data(file)).unwrap();
    normalize(&mut raw);
    let typed: T = store::read(&data(file)).unwrap();
    let back = serde_json::to_value(&typed).unwrap();
    if let Some((path, a, b)) = first_difference(&raw, &back, String::new()) {
        panic!("{file} does not round-trip at {path}: {a} became {b}");
    }
}

fn first_difference(a: &Value, b: &Value, path: String) -> Option<(String, Value, Value)> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => x
            .keys()
            .chain(y.keys())
            .find_map(|k| first_difference(x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), format!("{path}.{k}"))),
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            x.iter().zip(y).enumerate().find_map(|(i, (p, q))| first_difference(p, q, format!("{path}[{i}]")))
        }
        (Value::Number(x), Value::Number(y)) if x.as_f64() == y.as_f64() => None,
        _ => (a != b).then(|| (path, a.clone(), b.clone())),
    }
}

#[test]
fn collection_round_trips() {
    assert_lossless::<Collection>(store::COLLECTION, |v| {
        for e in v["entries"].as_array_mut().unwrap() {
            e.as_object_mut().unwrap().entry("date_note").or_insert(Value::Null);
        }
    });
}

#[test]
fn graph_round_trips() {
    assert_lossless::<Graph>(store::GRAPH, |_| {});
}

#[test]
fn civilizations_round_trip() {
    assert_lossless::<Civilizations>(store::CIVILIZATIONS, |_| {});
}
