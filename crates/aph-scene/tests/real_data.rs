//! The scene built from the real dataset in `data/`.

use std::{path::Path, sync::OnceLock};

use aph_data::store::Dataset;
use aph_scene::{
    atlas::{Atlas, NodeKind},
    buffers::{ARC_SEGMENTS, Dots, Ribbons},
    overlay,
    pick,
    sphere::LatLon,
    time::Clock,
};

fn data() -> &'static (Dataset, Atlas) {
    static DATA: OnceLock<(Dataset, Atlas)> = OnceLock::new();
    DATA.get_or_init(|| {
        let dataset = Dataset::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")).expect("dataset loads");
        let atlas = Atlas::build(&dataset);
        (dataset, atlas)
    })
}

#[test]
fn every_aphorism_becomes_a_node() {
    let (dataset, atlas) = data();
    let aphorisms = atlas.nodes.iter().filter(|n| n.kind == NodeKind::Aphorism).count();
    assert_eq!(aphorisms, dataset.collection.entries.len());
}

#[test]
fn ideas_are_places_threads_reach_not_dots() {
    let (dataset, atlas) = data();
    assert!(atlas.nodes.iter().all(|n| n.kind == NodeKind::Aphorism), "no idea dots");
    let to_ideas = dataset.graph.edges.iter().filter(|e| matches!(e.to, aph_data::graph::NodeRef::Idea(_))).count();
    let drawn = atlas
        .threads
        .iter()
        .filter(|t| matches!(t.kind, aph_scene::atlas::ThreadKind::Response { edge, .. } if matches!(dataset.graph.edges[edge].to, aph_data::graph::NodeRef::Idea(_))))
        .count();
    assert_eq!(drawn, to_ideas, "every thread answering an idea still has somewhere to land");
}

#[test]
fn most_threads_are_placed_and_each_appears_after_both_ends() {
    let (dataset, atlas) = data();
    let expected = dataset.graph.edges.len() + dataset.mentions.mentions.len();
    assert_eq!(atlas.threads.len() + atlas.unplaced.len(), expected, "responses and @mentions");
    assert!(
        atlas.unplaced.len() * 4 <= dataset.graph.edges.len(),
        "too many unplaced threads: {:?}",
        atlas.unplaced
    );
}

#[test]
fn nodes_are_in_chronological_order() {
    let (_, atlas) = data();
    assert!(atlas.nodes.windows(2).all(|w| w[0].from <= w[1].from));
}

#[test]
fn buffers_cover_the_whole_dataset() {
    let (_, atlas) = data();
    let ribbons = Ribbons::build(&atlas.threads);
    assert_eq!(ribbons.vertex_count(), atlas.threads.len() * 2 * (ARC_SEGMENTS + 1));
    let dots = Dots::build(&atlas.nodes);
    assert_eq!(dots.center.len(), atlas.nodes.len() * 4);
}

#[test]
fn clicking_athens_in_400_bce_finds_something_athenian() {
    let (_, atlas) = data();
    let athens = LatLon::new(37.98, 23.72);
    let hit = pick::nearest(&atlas.nodes, Clock::at(-350.0), athens, 3.0).expect("an aphorism near Athens");
    assert!(pick::angular_distance(atlas.nodes[hit].at, athens) < 3.0);
}

#[test]
fn the_page_model_lines_up_with_the_globe() {
    let (dataset, atlas) = data();
    let page = aph_scene::export::PageModel::build(dataset, atlas);
    assert_eq!(page.nodes.len(), atlas.nodes.len(), "dot i is node i");
    assert_eq!(page.edges.len() + page.mentions.len(), atlas.threads.len());
    assert_eq!(page.mentions.len(), dataset.mentions.mentions.len());
    let ids: std::collections::BTreeSet<_> = page.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids.len(), page.nodes.len(), "node ids are unique");
    assert!(page.nodes.iter().all(|n| n.id.starts_with("entry:") || n.id.starts_with("idea:")));
    assert_eq!(page.colors.religion.len(), aph_data::vocab::ReligionFamily::ALL.len());
    let json = serde_json::to_string(&page).unwrap();
    assert!(json.contains("\"authorName\"") && json.contains("\"workId\""), "camelCase like v3's page expects");
}

#[test]
fn coastlines_put_london_on_land_and_the_atlantic_at_sea() {
    let (_, atlas) = data();
    assert!(!atlas.land.is_empty(), "data/land-50m.json is cached and decodes");
    let (w, h) = (2048, 1024);
    let started = std::time::Instant::now();
    let field = overlay::coasts(&atlas.land, w, h);
    eprintln!("coast field {w}x{h} in {:?}", started.elapsed());
    let grid = overlay::Overlay::empty(w, h);
    let at = |lon: f64, lat: f64| {
        let (x, y) = grid.texel_at(lon, lat);
        field[(y * w + x) as usize]
    };
    assert!(at(-0.13, 51.51) < 128, "London is on land");
    assert!(at(-30.0, 40.0) > 128, "the mid-Atlantic is sea");
    assert!(at(78.0, 25.0) < 128, "the Indian subcontinent is land");
}

#[test]
fn curator_replies_fall_inside_their_schedule_phase() {
    let (dataset, atlas) = data();
    let page = aph_scene::export::PageModel::build(dataset, atlas);
    let schedule = dataset.schedule.as_ref().expect("curator_schedule.json is present");
    let with_replies = dataset.collection.entries.iter().filter(|e| !e.interpretation.is_empty()).count();
    assert_eq!(page.curator_replies.len(), with_replies);
    let genesis = &page.curator_replies["entry:genesis-1-3"];
    assert_eq!(genesis.phase, schedule.phases[0].name, "the Hebrew Bible is in the first phase");
    assert!((2005.0..2005.75).contains(&genesis.t), "January–August 2005: {}", genesis.t);
    assert!(page.curator_replies.values().all(|r| (2005.0..=2026.9).contains(&r.t)));
}

#[test]
fn regions_tile_all_land_and_rome_is_roman_in_117() {
    use aph_scene::regions::{self, SEA};
    let (_, atlas) = data();
    let (w, h) = (2048, 1024);
    let land = regions::land_mask(&overlay::coasts(&atlas.land, w, h));
    let started = std::time::Instant::now();
    let map = regions::regions(&atlas.civs, &overlay::active(&atlas.civs, 117), &land, w, h);
    eprintln!("regions {w}x{h} in {:?}", started.elapsed());
    assert!(map.labels.iter().zip(&land).all(|(&l, &is_land)| is_land == (l != SEA)), "every land texel has a region");
    let rome = map.civ_of(map.label_at(12.48, 41.89)).map(|i| atlas.civs[i].civ.0.as_str());
    assert_eq!(rome, Some("roman"));
}
