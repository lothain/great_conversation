//! Semantic checks that the type system can't express: cross-references, ordering, geometry.
//! Anything structural (dates, coordinates, vocabularies) is already rejected while parsing.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::{
    civ::{self, Civilizations},
    collection::{Collection, Quote},
    gloss,
    graph::{Graph, NodeRef},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub severity: Severity,
    pub at: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warn ",
        };
        write!(f, "{tag} {}: {}", self.at, self.message)
    }
}

#[derive(Default)]
pub struct Report(pub Vec<Problem>);

impl Report {
    fn push(&mut self, severity: Severity, at: impl fmt::Display, message: impl fmt::Display) {
        self.0.push(Problem { severity, at: at.to_string(), message: message.to_string() });
    }
    fn error(&mut self, at: impl fmt::Display, message: impl fmt::Display) {
        self.push(Severity::Error, at, message);
    }
    fn warn(&mut self, at: impl fmt::Display, message: impl fmt::Display) {
        self.push(Severity::Warning, at, message);
    }
    #[must_use]
    pub fn count(&self, severity: Severity) -> usize {
        self.0.iter().filter(|p| p.severity == severity).count()
    }
}

pub fn collection(c: &Collection, report: &mut Report) {
    let sections: BTreeSet<_> = c.sections.iter().map(|s| &s.id).collect();
    let mut seen = BTreeSet::new();

    for (id, author) in &c.authors {
        for (stage, residence) in author.places.iter().enumerate() {
            if !residence.when.overlaps(author.life) {
                report.warn(format!("author {id} place[{stage}]"), format!("{} is outside life {}", residence.when, author.life));
            }
        }
        for (stage, faith) in author.religion.iter().enumerate() {
            if !faith.when.overlaps(author.life) {
                report.warn(format!("author {id} religion[{stage}]"), format!("{} is outside life {}", faith.when, author.life));
            }
        }
    }

    for e in &c.entries {
        let at = format!("entry {}", e.id);
        if !seen.insert(&e.id) {
            report.error(&at, "duplicate id");
        }
        if !sections.contains(&e.section) {
            report.error(&at, format!("unknown section {}", e.section));
        }
        match c.authors.get(&e.author) {
            None => report.error(&at, format!("unknown author {}", e.author)),
            Some(author) if !author.places.iter().any(|r| e.date.overlaps(r.when)) => {
                report.warn(&at, format!("date {} falls in no residence stage of {}", e.date, e.author));
            }
            Some(_) => {}
        }
        for (i, q) in e.quotes.iter().enumerate() {
            quote(q, &format!("{at} quote[{i}]"), report);
        }
        let glossable = [Some(e.context.as_str()), Some(e.interpretation.as_str())]
            .into_iter()
            .chain(e.quotes.iter().map(|q| q.trans.as_deref()))
            .flatten();
        for text in glossable {
            for g in gloss::scan(text) {
                if let Err(err) = g {
                    report.error(&at, err);
                }
            }
        }
    }
}

fn quote(q: &Quote, at: &str, report: &mut Report) {
    match (q.is_latin_script(), &q.translit) {
        (false, None) => report.error(at, "non-Latin original needs a transliteration"),
        (true, Some(_)) => report.warn(at, "transliteration given for a Latin-script original"),
        _ => {}
    }
    if q.lang != "en" && q.trans.is_none() {
        report.error(at, format!("{} original needs a translation", q.lang));
    }
    if gloss::scan(&q.orig).any(|g| g.is_ok()) {
        report.error(at, "glosses belong in context, translation or interpretation, never the original");
    }
}

pub fn graph(g: &Graph, report: &mut Report) {
    for (id, person) in &g.people {
        let mut slugs = BTreeSet::new();
        for w in &person.works {
            if !slugs.insert(&w.id) {
                report.error(format!("person {id}"), format!("duplicate work {}", w.id));
            }
            for co in &w.coauthors {
                if !g.people.contains_key(co) {
                    report.error(format!("work {id}/{}", w.id), format!("unknown coauthor {co}"));
                }
            }
        }
        if person.works.windows(2).any(|w| w[0].date.earliest() > w[1].date.earliest()) {
            report.error(format!("person {id}"), "works are not in date order");
        }
    }
    for (id, idea) in &g.ideas {
        for p in idea.people.iter().filter(|p| !g.people.contains_key(*p)) {
            report.error(format!("idea {id}"), format!("unknown person {p}"));
        }
    }
    for (i, edge) in g.edges.iter().enumerate() {
        let at = format!("edge[{i}] {} -> {}", edge.from, edge.to);
        for end in [&edge.from, &edge.to] {
            if !g.resolves(end) {
                report.error(&at, format!("{end} does not resolve"));
            }
        }
        if edge.from == edge.to {
            report.error(&at, "a work cannot respond to itself");
        }
        // A response can't predate what it responds to (people and ideas span lifetimes, so only works).
        if let (NodeRef::Work(_), NodeRef::Work(_)) = (&edge.from, &edge.to)
            && let (Some(from), Some(to)) = (g.date_of(&edge.from), g.date_of(&edge.to))
            && from.latest().is_some_and(|end| end < to.earliest())
        {
            report.error(&at, format!("response ({from}) predates its target ({to})"));
        }
        if edge.aspect.split_whitespace().count() > 15 {
            report.warn(&at, "aspect is over 15 words");
        }
    }
}

pub fn cross(c: &Collection, g: &Graph, report: &mut Report) {
    for (id, author) in &c.authors {
        match g.people.get(id) {
            None => report.error(format!("author {id}"), "missing from the graph manifest"),
            Some(p) if !p.in_collection => report.error(format!("person {id}"), "in the collection but marked in_collection: false"),
            Some(p) if p.life != author.life => report.warn(format!("person {id}"), format!("life {} differs from collection {}", p.life, author.life)),
            Some(_) => {}
        }
    }
    for (id, p) in &g.people {
        if p.in_collection && !c.authors.contains_key(id) {
            report.error(format!("person {id}"), "marked in_collection but not a collection author");
        }
    }
    for e in &c.entries {
        if let Some(key) = &e.work_id {
            if key.person != e.author {
                report.error(format!("entry {}", e.id), format!("work_id {key} belongs to another author"));
            } else if g.work(key).is_none() {
                report.error(format!("entry {}", e.id), format!("work_id {key} is not in the graph"));
            }
        }
    }
}

pub fn civilizations(civs: &Civilizations, report: &mut Report) {
    let mut snapshots: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (i, f) in civs.features.iter().enumerate() {
        let p = &f.properties;
        let at = format!("feature[{i}] {} {}", p.civ, p.when);
        snapshots.entry(&p.civ).or_default().push(p.when);

        if !p.when.within(p.valid) {
            report.error(&at, format!("snapshot {} is outside valid {}", p.when, p.valid));
        }
        let total: f64 = p.religions.iter().map(|r| r.share).sum();
        if total > 1.01 || p.religions.iter().any(|r| !(0.0..=1.0).contains(&r.share)) {
            report.error(&at, format!("religion shares invalid (sum {total:.2})"));
        }
        for (label, polygons) in [("core", f.geometry.polygons()), ("halo", p.halo.polygons())] {
            for ring in polygons.iter().flatten() {
                if ring.len() < 4 || ring.first() != ring.last() {
                    report.error(&at, format!("{label} ring is not closed"));
                } else if civ::self_intersects(ring) {
                    report.error(&at, format!("{label} ring self-intersects"));
                }
            }
            for outer in polygons.iter().filter_map(|poly| poly.first()) {
                if civ::signed_area2(outer) <= 0.0 {
                    report.warn(&at, format!("{label} outer ring is not counter-clockwise"));
                }
            }
        }
        let outside = f.geometry.polygons().iter().flatten().flatten().filter(|v| !p.halo.contains(**v)).count();
        if outside > 0 {
            report.error(&at, format!("{outside} core vertices lie outside the halo"));
        }
    }
    for (civ, whens) in snapshots {
        if whens.windows(2).any(|w| w[0].earliest() > w[1].earliest()) {
            report.warn(format!("civ {civ}"), "snapshots are not in date order");
        }
    }
}
