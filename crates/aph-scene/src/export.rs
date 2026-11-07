//! The model the HTML page renders (feed, cards, legends), in the shape v3's page reads.
//! This replaces v3's `build.py`: one resolver, in Rust, for the globe and the page alike.

use std::collections::BTreeMap;

use aph_data::{graph::NodeRef, store::Dataset, vocab::ReligionFamily};
use serde::Serialize;

use crate::{
    atlas::{Atlas, NodeKind, Source, ThreadKind, religion_stage},
    palette,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageModel {
    pub title: String,
    pub subtitle: String,
    pub curator: String,
    /// In document order.
    pub sections: Ordered,
    pub authors: BTreeMap<String, AuthorCard>,
    /// Chronological; index i is node i of the globe (dots are addressed by it).
    pub nodes: Vec<PageNode>,
    pub edges: Vec<PageEdge>,
    pub civs: Vec<PageCiv>,
    /// @mentions between utterances.
    pub mentions: Vec<PageMention>,
    /// Full source records by node id, so a channel can be downloaded without losing fields.
    pub entries: BTreeMap<String, aph_data::collection::Entry>,
    /// When the curator wrote each reply (by node id), from `curator_schedule.json`.
    pub curator_replies: BTreeMap<String, CuratorReply>,
    pub curator_profile: Option<aph_data::schedule::CuratorProfile>,
    pub colors: Colors,
}

#[derive(Debug, Serialize)]
pub struct PageMention {
    pub from: String,
    pub to: String,
    pub kind: aph_data::mentions::MentionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phrase: Option<String>,
    pub note: String,
    pub basis: aph_data::graph::Basis,
    pub conf: aph_data::vocab::Confidence,
    pub t: i32,
    pub a: [f64; 2],
    pub b: [f64; 2],
}

#[derive(Debug, Serialize)]
pub struct CuratorReply {
    /// Fractional year, e.g. 2005.667 for September 2005.
    pub t: f64,
    pub phase: String,
}

#[derive(Debug, Serialize)]
pub struct AuthorCard {
    pub name: String,
    pub life: String,
    pub places: Vec<Stage>,
    pub religion: Vec<Faith>,
    pub role: Option<String>,
    pub portrait: Option<PortraitCard>,
}

#[derive(Debug, Serialize)]
pub struct Stage {
    pub when: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

/// A licensed portrait thumbnail and its attribution.
#[derive(Debug, Serialize)]
pub struct PortraitCard {
    pub thumb: String,
    pub license: Option<String>,
    pub license_url: Option<String>,
    pub artist: Option<String>,
    pub commons: String,
}

#[derive(Debug, Serialize)]
pub struct Faith {
    pub when: String,
    pub family: ReligionFamily,
    pub detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNode {
    pub id: String,
    pub kind: &'static str,
    /// When it appears: spread across the source's date range, refined by month and day.
    pub t: f64,
    /// The real span.
    pub t0: i32,
    pub t1: i32,
    pub lat: f64,
    pub lon: f64,
    pub place_name: String,
    pub author: Option<String>,
    pub author_name: String,
    pub section: Option<String>,
    pub work: String,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub date: String,
    pub context: Option<String>,
    pub interp: Option<String>,
    pub orig: Option<String>,
    pub trans: Option<String>,
    pub lang: Option<String>,
    pub religion: Option<ReligionFamily>,
    pub rel_detail: Option<String>,
    pub url: Option<String>,
    pub source_label: String,
    pub trans_by: Option<String>,
    pub work_id: Option<String>,
    pub speaker: Option<aph_data::collection::Speaker>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageEdge {
    pub from: String,
    pub to: String,
    pub t: i32,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub from_label: String,
    pub to_label: String,
    pub kind: aph_data::graph::EdgeKind,
    pub valence: aph_data::graph::Valence,
    pub aspect: String,
    pub quote: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PageCiv {
    pub civ: String,
    pub name: String,
    pub t0: i32,
    pub t1: i32,
    pub religions: Vec<aph_data::civ::ReligionShare>,
    pub note: String,
    pub core: Vec<aph_data::civ::Polygon>,
    pub halo: Vec<aph_data::civ::Polygon>,
    pub color: String,
}

/// CSS colours, so the page's swatches match the globe exactly.
#[derive(Debug, Serialize)]
pub struct Colors {
    pub religion: BTreeMap<ReligionFamily, String>,
    /// Bit i of a religion mask is `religion_order[i]`.
    pub religion_order: Vec<ReligionFamily>,
    pub valence: BTreeMap<i8, String>,
    pub ink: String,
    pub gold: String,
}

fn label(data: &Dataset, node: &NodeRef) -> String {
    let g = &data.graph;
    match node {
        NodeRef::Work(key) => g.work(key).map_or_else(|| key.to_string(), |w| w.title.clone()),
        NodeRef::Person(id) => g.people.get(id).map_or_else(|| id.to_string(), |p| p.name.clone()),
        NodeRef::Idea(id) => g.ideas.get(id).map_or_else(|| id.to_string(), |i| i.name.clone()),
    }
}

impl PageModel {
    #[must_use]
    pub fn build(data: &Dataset, atlas: &Atlas) -> Self {
        let (c, g) = (&data.collection, &data.graph);

        let nodes = atlas
            .nodes
            .iter()
            .map(|n| match &n.source {
                Source::Entry(i) => {
                    let e = &c.entries[*i];
                    let author = c.authors.get(&e.author);
                    let faith = author.and_then(|a| religion_stage(a, i32::midpoint(n.from, n.until)));
                    let quote = e.quotes.first();
                    PageNode {
                        id: n.id(c),
                        kind: "entry",
                        t: n.moment,
                        t0: n.span.0,
                        t1: n.span.1,
                        lat: n.at.lat,
                        lon: n.at.lon,
                        place_name: e.place.name.clone(),
                        author: Some(e.author.to_string()),
                        author_name: author.map_or_else(|| e.author.to_string(), |a| a.name.clone()),
                        section: Some(e.section.to_string()),
                        work: e.work.clone(),
                        reference: e.reference.clone(),
                        date: e.date.to_string(),
                        context: Some(e.context.clone()),
                        interp: Some(e.interpretation.clone()),
                        orig: quote.map(|q| q.orig.clone()),
                        trans: quote.and_then(|q| q.trans.clone()),
                        lang: quote.map(|q| q.lang.clone()),
                        religion: Some(faith.map_or(ReligionFamily::Unknown, |f| f.family)),
                        rel_detail: faith.map(|f| f.detail.clone()),
                        url: e.source.url.clone(),
                        source_label: e.source.label.clone(),
                        trans_by: quote.and_then(|q| q.trans_by.clone()),
                        work_id: e.work_id.as_ref().map(|k| format!("work:{k}")),
                        speaker: e.speaker.clone(),
                    }
                }
                Source::Idea(id) => {
                    let idea = &g.ideas[id];
                    let people: Vec<_> = idea.people.iter().map(|p| g.people.get(p).map_or_else(|| p.to_string(), |p| p.name.clone())).collect();
                    PageNode {
                        id: n.id(c),
                        kind: if n.kind == NodeKind::Idea { "idea" } else { "entry" },
                        t: n.moment,
                        t0: n.span.0,
                        t1: n.span.1,
                        lat: n.at.lat,
                        lon: n.at.lon,
                        place_name: String::new(),
                        author: None,
                        author_name: if people.is_empty() { "—".into() } else { people.join(", ") },
                        section: None,
                        work: idea.name.clone(),
                        reference: None,
                        date: idea.date.to_string(),
                        context: None,
                        interp: Some(idea.summary.clone()),
                        orig: None,
                        trans: None,
                        lang: None,
                        religion: None,
                        rel_detail: None,
                        url: None,
                        source_label: String::new(),
                        trans_by: None,
                        work_id: None,
                        speaker: None,
                    }
                }
            })
            .collect();

        let edges = atlas
            .threads
            .iter()
            .filter_map(|t| match t.kind {
                ThreadKind::Response { edge, .. } => Some((t, edge)),
                ThreadKind::Mention { .. } => None,
            })
            .map(|(t, edge)| {
                let e = &g.edges[edge];
                PageEdge {
                    from: e.from.to_string(),
                    to: e.to.to_string(),
                    t: t.year,
                    a: [t.from.lat, t.from.lon],
                    b: [t.to.lat, t.to.lon],
                    from_label: label(data, &e.from),
                    to_label: label(data, &e.to),
                    kind: e.kind,
                    valence: e.valence,
                    aspect: e.aspect.clone(),
                    quote: e.quote.as_ref().map(|q| q.trans.clone().unwrap_or_else(|| q.orig.clone())),
                }
            })
            .collect();

        let civs = atlas
            .civs
            .iter()
            .map(|s| PageCiv {
                civ: s.civ.to_string(),
                name: s.name.clone(),
                t0: s.from,
                t1: s.until,
                religions: s.religions.clone(),
                note: s.note.clone(),
                core: s.core.clone(),
                halo: s.halo.clone(),
                color: palette::civ(&s.civ.0).css(),
            })
            .collect();

        let authors = c
            .authors
            .iter()
            .map(|(id, a)| {
                let card = AuthorCard {
                    name: a.name.clone(),
                    life: a.life.to_string(),
                    places: a.places.iter().map(|p| Stage { when: p.when.to_string(), name: p.name.clone(), lat: p.lat.into(), lon: p.lon.into() }).collect(),
                    religion: a.religion.iter().map(|r| Faith { when: r.when.to_string(), family: r.family, detail: r.detail.clone() }).collect(),
                    role: g.people.get(id).and_then(|p| p.role.clone()),
                    portrait: data.portraits.get(id).map(|p| PortraitCard {
                        thumb: p.thumb.clone(),
                        license: p.license.clone(),
                        license_url: p.license_url.clone(),
                        artist: p.artist.clone(),
                        commons: p.commons.clone(),
                    }),
                };
                (id.to_string(), card)
            })
            .collect();

        let religion = ReligionFamily::ALL.iter().map(|&f| (f, palette::religion(f).css())).collect();
        let valence = [aph_data::graph::Valence::Rejects, aph_data::graph::Valence::Mixed, aph_data::graph::Valence::Adopts]
            .into_iter()
            .map(|v| (i8::from(v), palette::valence(v).css()))
            .collect();

        Self {
            title: c.title.clone(),
            subtitle: c.subtitle.clone(),
            curator: c.author.clone(),
            sections: Ordered(c.sections.iter().map(|s| (s.id.to_string(), s.name.clone())).collect()),
            authors,
            nodes,
            edges,
            civs,
            mentions: mentions(data, atlas),
            entries: c.entries.iter().map(|e| (format!("entry:{}", e.id), e.clone())).collect(),
            curator_replies: curator_replies(data, atlas),
            curator_profile: data.schedule.as_ref().map(|s| s.profile.clone()),
            colors: Colors { religion, religion_order: ReligionFamily::ALL.to_vec(), valence, ink: palette::INK.css(), gold: palette::GOLD.css() },
        }
    }
}

fn mentions(data: &Dataset, atlas: &Atlas) -> Vec<PageMention> {
    atlas
        .threads
        .iter()
        .filter_map(|t| match t.kind {
            ThreadKind::Mention { mention } => Some((t, &data.mentions.mentions[mention])),
            ThreadKind::Response { .. } => None,
        })
        .map(|(t, m)| PageMention {
            from: format!("entry:{}", m.from),
            to: format!("entry:{}", m.to),
            kind: m.kind,
            phrase: m.phrase.clone(),
            note: m.note.clone(),
            basis: m.basis,
            conf: m.conf,
            t: t.year,
            a: [t.from.lat, t.from.lon],
            b: [t.to.lat, t.to.lon],
        })
        .collect()
}

/// A month's start as a fractional year: 2005-09 → 2005.667.
fn month_year((year, month, _): aph_data::edtf::Day) -> f64 {
    f64::from(year) + f64::from(month.saturating_sub(1)) / 12.0
}

/// When the curator wrote each reply. Each author's replies belong to the first schedule phase
/// that lists them (else the `rest` phase); within a phase, replies are spread across its months
/// in the order the utterances were said, with a small stable jitter.
fn curator_replies(data: &Dataset, atlas: &Atlas) -> BTreeMap<String, CuratorReply> {
    let Some(schedule) = &data.schedule else { return BTreeMap::new() };
    let c = &data.collection;
    let mut phases: BTreeMap<usize, Vec<(f64, String)>> = BTreeMap::new();
    for n in &atlas.nodes {
        if let Source::Entry(i) = n.source {
            let e = &c.entries[i];
            if !e.interpretation.is_empty() {
                phases.entry(schedule.phase_of(&e.author)).or_default().push((n.moment, n.id(c)));
            }
        }
    }
    let mut out = BTreeMap::new();
    for (phase, mut replies) in phases {
        let (name, when) = schedule.phase(phase);
        let start = month_year(when.earliest());
        let end = when.latest().map_or(f64::from(crate::atlas::PRESENT), month_year) + 1.0 / 12.0;
        replies.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let k = replies.len() as f64;
        for (j, (_, id)) in replies.into_iter().enumerate() {
            let slot = (j as f64 + 0.2 + 0.6 * crate::atlas::jitter(&format!("cur:{id}"))) / k;
            let t = ((start + slot * (end - start)) * 1000.0).round() / 1000.0;
            out.insert(id, CuratorReply { t, phase: name.to_owned() });
        }
    }
    out
}

/// A JSON object whose keys keep their given order (a `BTreeMap` would sort them).
#[derive(Debug)]
pub struct Ordered(pub Vec<(String, String)>);

impl Serialize for Ordered {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}
