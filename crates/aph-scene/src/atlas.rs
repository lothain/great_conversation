//! The dataset resolved into things that can be drawn: located aphorisms, ideas,
//! threads between them, and civilization snapshots. Built once at startup.

use std::collections::BTreeMap;

use aph_data::{
    civ::{Feature, Polygon, ReligionShare},
    collection::{Author, Collection, ReligionStage, Residence},
    edtf::Edtf,
    graph::{EdgeKind, Graph, NodeRef, Valence},
    ids::{CivId, IdeaId, PersonId, WorkKey},
    store::Dataset,
    vocab::ReligionFamily,
};

use crate::sphere::LatLon;

/// A year on the astronomical calendar (1 BCE = 0).
pub type Year = i32;

/// The last year shown.
pub const PRESENT: Year = 2026;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Aphorism,
    Idea,
}

/// Where a node came from in the dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Index into `collection.entries`.
    Entry(usize),
    Idea(IdeaId),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub source: Source,
    pub kind: NodeKind,
    pub at: LatLon,
    /// When it appears on the globe and in the feed (see [`spread`]).
    pub from: Year,
    /// The end of its real span; it stays visible relative to this.
    pub until: Year,
    /// The real (possibly widened) span of the source's date.
    pub span: (Year, Year),
    /// The precise moment it appears, as a fractional year: `from`, refined by the month and
    /// day of an exact date (so 1945-07-16 comes before 1945-12). Dots and the feed use this.
    pub moment: f64,
    pub label: String,
    pub religion: Option<ReligionFamily>,
}

/// What a thread draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadKind {
    /// A `graph.json` response: index into `graph.edges`.
    Response { edge: usize, kind: EdgeKind, valence: Valence },
    /// An  between utterances: index into `mentions.mentions`.
    Mention { mention: usize },
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub kind: ThreadKind,
    pub from: LatLon,
    pub to: LatLon,
    /// The year it appears: for a response the later of its two ends, for a mention when the
    /// mentioning utterance is heard.
    pub year: Year,
}

#[derive(Debug, Clone)]
pub struct CivSnapshot {
    pub civ: CivId,
    pub name: String,
    pub from: Year,
    pub until: Year,
    pub core: Vec<Polygon>,
    pub halo: Vec<Polygon>,
    pub religions: Vec<ReligionShare>,
    pub note: String,
}

#[derive(Debug)]
pub struct Atlas {
    /// Chronological by `from`.
    pub nodes: Vec<Node>,
    pub threads: Vec<Thread>,
    pub civs: Vec<CivSnapshot>,
    /// Edges whose endpoints have no known location.
    pub unplaced: Vec<String>,
    /// Land polygons, for coastlines (empty when the land file is absent).
    pub land: Vec<Polygon>,
}

/// (first, last) year of a date or span.
pub(crate) fn years(e: Edtf) -> (Year, Year) {
    (e.start_year(), e.latest().map_or(e.start_year(), |(y, _, _)| y))
}

fn located_at(places: &[Residence], year: Year) -> Option<LatLon> {
    let stage = places
        .iter()
        .find(|p| {
            let (a, b) = years(p.when);
            (a..=b).contains(&year)
        })
        .or_else(|| places.iter().rev().find(|p| years(p.when).0 <= year))
        .or_else(|| places.first())?;
    Some(LatLon::new(stage.lat, stage.lon))
}

pub(crate) fn religion_stage(author: &Author, year: Year) -> Option<&ReligionStage> {
    author
        .religion
        .iter()
        .find(|r| {
            let (a, b) = years(r.when);
            (a..=b).contains(&year)
        })
        .or_else(|| author.religion.iter().rev().find(|r| years(r.when).0 <= year))
        .or_else(|| author.religion.first())
}

/// A stable pseudo-random number in 0..=1 from a string (FNV-1a).
pub(crate) fn jitter(key: &str) -> f64 {
    let h = key.bytes().fold(0x811c_9dc5_u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    f64::from(h) / f64::from(u32::MAX)
}

/// Many entries share one date range (all of Genesis is -0999/-0399). Rather than all
/// appearing in its first year, place a source's entries in document order across the
/// range, each nudged by a stable jitter. Approximate single years (`0095~`) get ±10 years.
/// Exact dates keep their month and day: the fractional year of a date's start, if it has a month.
fn precise_start(date: Edtf) -> Option<f64> {
    let (y, m, d) = date.earliest();
    let has_month = match date {
        Edtf::Point(p) | Edtf::Interval { start: p, .. } => p.date.month.is_some(),
    };
    has_month.then(|| f64::from(y) + f64::from(m - 1) / 12.0 + f64::from(d - 1) / 365.0)
}

fn spread(nodes: &mut [Node], collection: &Collection) {
    let mut groups: BTreeMap<(&str, &str, String), Vec<usize>> = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Source::Entry(e) = n.source {
            let e = &collection.entries[e];
            groups.entry((e.author.0.as_str(), e.work.as_str(), e.date.to_string())).or_default().push(i);
        }
    }
    for ((_, _, date), members) in groups {
        let (mut s0, mut s1) = nodes[members[0]].span;
        if s0 == s1 && date.contains('~') {
            (s0, s1) = (s0 - 10, s1 + 10);
        }
        let k = members.len() as f64;
        for (slot, &i) in members.iter().enumerate() {
            let n = &mut nodes[i];
            n.span = (s0, s1);
            n.until = s1;
            n.from = if s0 == s1 {
                s0
            } else {
                let Source::Entry(e) = n.source else { unreachable!("grouped entries only") };
                let entry = &collection.entries[e];
                let place = (slot as f64 + 0.15 + 0.7 * jitter(&entry.id.0)) / k;
                let t = (f64::from(s0) + place * f64::from(s1 - s0)).round() as Year;
                // Nobody speaks after they die.
                let death = collection.authors.get(&entry.author).and_then(|a| a.life.latest()).map(|(y, _, _)| y);
                death.filter(|&d| s0 <= d && d < t).unwrap_or(t)
            };
            n.moment = f64::from(n.from);
        }
    }
}

/// The civilization data stops between 1950 and 1991. Carry the last snapshot of polities
/// that still exist in roughly that shape forward to the present, so today isn't blank.
/// Remove once the data has modern snapshots.
const EXTEND_TO_PRESENT: [(&str, &str); 2] = [("usa", "United States"), ("western_bloc", "NATO / EU Europe")];

fn extend_to_present(civs: &mut Vec<CivSnapshot>) {
    for (key, name) in EXTEND_TO_PRESENT {
        let last = civs.iter().filter(|c| c.civ.0 == key).max_by_key(|c| c.until).cloned();
        if let Some(last) = last.filter(|c| c.until < PRESENT) {
            civs.push(CivSnapshot {
                name: name.to_owned(),
                from: last.until + 1,
                until: PRESENT,
                note: format!("{} (Extended to the present by the viewer.)", last.note),
                ..last
            });
        }
    }
}

struct Resolver<'a> {
    collection: &'a Collection,
    graph: &'a Graph,
    /// Where and when each work first appears as an aphorism.
    works: BTreeMap<&'a WorkKey, (LatLon, Year)>,
    ideas: BTreeMap<String, (LatLon, Year)>,
}

impl Resolver<'_> {
    fn person_at(&self, person: &PersonId, year: Year) -> Option<LatLon> {
        let places = match self.collection.authors.get(person) {
            Some(author) => &author.places,
            None => &self.graph.people.get(person)?.places,
        };
        located_at(places, year)
    }

    fn endpoint(&self, node: &NodeRef) -> Option<(LatLon, Year)> {
        match node {
            NodeRef::Work(key) => self.works.get(key).copied().or_else(|| {
                let year = self.graph.work(key)?.date.start_year();
                Some((self.person_at(&key.person, year)?, year))
            }),
            NodeRef::Person(id) => {
                let (born, died) = years(self.graph.people.get(id)?.life);
                Some((self.person_at(id, i32::midpoint(born, died))?, born + 20))
            }
            NodeRef::Idea(id) => self.ideas.get(&id.0).copied(),
        }
    }
}

impl Atlas {
    #[must_use]
    pub fn build(data: &Dataset) -> Self {
        let (collection, graph) = (&data.collection, &data.graph);

        let mut nodes: Vec<Node> = collection
            .entries
            .iter()
            .enumerate()
            .map(|(index, e)| {
                let (from, until) = years(e.date);
                let author = collection.authors.get(&e.author);
                Node {
                    source: Source::Entry(index),
                    kind: NodeKind::Aphorism,
                    at: LatLon::new(e.place.lat, e.place.lon),
                    from,
                    until,
                    span: (from, until),
                    moment: f64::from(from),
                    label: format!("{} — {}", author.map_or(e.author.0.as_str(), |a| a.name.as_str()), e.work),
                    religion: author.and_then(|a| religion_stage(a, i32::midpoint(from, until))).map(|r| r.family),
                }
            })
            .collect();
        spread(&mut nodes, collection);
        // Refine within the year only, never undoing the spreading.
        for n in &mut nodes {
            if let Source::Entry(i) = n.source
                && let Some(p) = precise_start(collection.entries[i].date)
                && p.trunc() as Year == n.from
            {
                n.moment = (p * 10_000.0).round() / 10_000.0;
            }
        }

        // A work's threads start where and when it is first heard.
        let mut works = BTreeMap::new();
        for n in &nodes {
            if let Source::Entry(i) = n.source
                && let Some(key) = &collection.entries[i].work_id
            {
                works.entry(key).and_modify(|w: &mut (LatLon, Year)| if n.from < w.1 { *w = (n.at, n.from) }).or_insert((n.at, n.from));
            }
        }
        let mut resolver = Resolver { collection, graph, works, ideas: BTreeMap::new() };

        for (id, idea) in &graph.ideas {
            let (from, _) = years(idea.date);
            let mut points: Vec<_> = idea.people.iter().filter_map(|p| resolver.person_at(p, from)).collect();
            if points.is_empty() {
                // Nobody behind it is located: place it among those who answer it.
                let target = NodeRef::Idea(id.clone());
                points = graph
                    .edges
                    .iter()
                    .filter(|e| e.to == target)
                    .filter_map(|e| resolver.endpoint(&e.from).map(|(at, _)| at))
                    .collect();
            }
            if let Some(at) = LatLon::centroid(&points) {
                resolver.ideas.insert(id.0.clone(), (at, from));
            }
        }

        let mut threads = Vec::new();
        let mut unplaced = Vec::new();
        for (index, edge) in graph.edges.iter().enumerate() {
            match (resolver.endpoint(&edge.from), resolver.endpoint(&edge.to)) {
                (Some((from, a)), Some((to, b))) => {
                    let kind = ThreadKind::Response { edge: index, kind: edge.kind, valence: edge.valence };
                    threads.push(Thread { kind, from, to, year: a.max(b) });
                }
                _ => unplaced.push(format!("{} -> {}", edge.from, edge.to)),
            }
        }

        // @mentions: one utterance calling back to another, appearing when the caller is heard.
        let by_entry: BTreeMap<&str, &Node> = nodes
            .iter()
            .filter_map(|n| match n.source {
                Source::Entry(i) => Some((collection.entries[i].id.0.as_str(), n)),
                Source::Idea(_) => None,
            })
            .collect();
        for (index, m) in data.mentions.mentions.iter().enumerate() {
            match (by_entry.get(m.from.0.as_str()), by_entry.get(m.to.0.as_str())) {
                (Some(a), Some(b)) => threads.push(Thread { kind: ThreadKind::Mention { mention: index }, from: a.at, to: b.at, year: a.from }),
                _ => unplaced.push(format!("mention {} -> {}", m.from, m.to)),
            }
        }

        let mut civs: Vec<CivSnapshot> = data.civilizations.features.iter().map(CivSnapshot::from).collect();
        extend_to_present(&mut civs);
        nodes.sort_by(|a, b| a.moment.total_cmp(&b.moment));
        Self { nodes, threads, civs, unplaced, land: data.land.clone() }
    }
}

impl Node {
    /// Stable id shared with the web page: `entry:<id>` or `idea:<id>`.
    #[must_use]
    pub fn id(&self, collection: &Collection) -> String {
        match &self.source {
            Source::Entry(i) => format!("entry:{}", collection.entries[*i].id),
            Source::Idea(id) => format!("idea:{id}"),
        }
    }
}

impl From<&Feature> for CivSnapshot {
    fn from(f: &Feature) -> Self {
        let p = &f.properties;
        let (from, until) = years(p.valid);
        Self {
            civ: p.civ.clone(),
            name: p.name.clone(),
            from,
            until,
            core: f.geometry.polygons().to_vec(),
            halo: p.halo.polygons().to_vec(),
            religions: p.religions.clone(),
            note: p.note.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_is_stable_and_in_range() {
        assert_eq!(jitter("genesis-1-3"), jitter("genesis-1-3"));
        assert!((0.0..=1.0).contains(&jitter("x")));
    }
}
