//! `graph.json`: the manifest of people and works, ideas, and `responds_to` edges.

use std::{collections::BTreeMap, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    collection::Residence,
    edtf::Edtf,
    ids::{IdeaId, PersonId, WorkKey, WorkSlug},
    vocab::Confidence,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub people: BTreeMap<PersonId, Person>,
    pub ideas: BTreeMap<IdeaId, Idea>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    pub name: String,
    pub life: Edtf,
    /// Residence timeline for people outside the collection (collection authors keep theirs in `aphorisms.json`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<Residence>,
    pub in_collection: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikipedia: Option<String>,
    /// Ordered by date.
    pub works: Vec<Work>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Work {
    pub id: WorkSlug,
    pub title: String,
    pub date: Edtf,
    pub kind: WorkKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coauthors: Vec<PersonId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkKind {
    Treatise,
    Essay,
    Book,
    Dialogue,
    Poem,
    Play,
    Novel,
    Letter,
    Lecture,
    Paper,
    Scripture,
    Film,
    Episode,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Idea {
    pub name: String,
    pub date: Edtf,
    pub people: Vec<PersonId>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: NodeRef,
    pub to: NodeRef,
    pub kind: EdgeKind,
    pub valence: Valence,
    /// What is accepted or rejected, ≤ 15 words.
    pub aspect: String,
    pub conf: Confidence,
    pub basis: Basis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<EdgeQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeQuote {
    pub lang: String,
    pub orig: String,
    pub trans: Option<String>,
    #[serde(rename = "ref")]
    pub reference: String,
}

/// Strength of evidence, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    Quotes,
    Responds,
    Cites,
    Alludes,
    Influenced,
}

/// The later work's stance toward the earlier one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "i8", into = "i8")]
pub enum Valence {
    Rejects = -1,
    Mixed = 0,
    Adopts = 1,
}

#[derive(Debug, thiserror::Error)]
#[error("valence must be -1, 0 or 1, not {0}")]
pub struct BadValence(i8);

impl TryFrom<i8> for Valence {
    type Error = BadValence;
    fn try_from(v: i8) -> Result<Self, Self::Error> {
        match v {
            -1 => Ok(Self::Rejects),
            0 => Ok(Self::Mixed),
            1 => Ok(Self::Adopts),
            other => Err(BadValence(other)),
        }
    }
}

impl From<Valence> for i8 {
    fn from(v: Valence) -> Self {
        v as i8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Basis {
    Explicit,
    Scholarly,
    Editorial,
}

/// `person:hume`, `work:kant/prolegomena`, `idea:hilbert-program`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum NodeRef {
    Person(PersonId),
    Work(WorkKey),
    Idea(IdeaId),
}

#[derive(Debug, thiserror::Error)]
pub enum BadNodeRef {
    #[error("`{0}`: expected `person:`, `work:` or `idea:` prefix")]
    Prefix(String),
    #[error(transparent)]
    Work(#[from] crate::ids::BadWorkKey),
}

impl FromStr for NodeRef {
    type Err = BadNodeRef;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once(':') {
            Some(("person", id)) if !id.is_empty() => Ok(Self::Person(PersonId(id.to_owned()))),
            Some(("idea", id)) if !id.is_empty() => Ok(Self::Idea(IdeaId(id.to_owned()))),
            Some(("work", key)) => Ok(Self::Work(key.parse()?)),
            _ => Err(BadNodeRef::Prefix(s.to_owned())),
        }
    }
}

impl TryFrom<String> for NodeRef {
    type Error = BadNodeRef;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl fmt::Display for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Person(id) => write!(f, "person:{id}"),
            Self::Work(key) => write!(f, "work:{key}"),
            Self::Idea(id) => write!(f, "idea:{id}"),
        }
    }
}

impl From<NodeRef> for String {
    fn from(n: NodeRef) -> Self {
        n.to_string()
    }
}

impl Graph {
    #[must_use]
    pub fn work(&self, key: &WorkKey) -> Option<&Work> {
        self.people.get(&key.person)?.works.iter().find(|w| w.id == key.slug)
    }

    /// The date a node can be said to exist from, if it resolves.
    #[must_use]
    pub fn date_of(&self, node: &NodeRef) -> Option<Edtf> {
        match node {
            NodeRef::Person(id) => self.people.get(id).map(|p| p.life),
            NodeRef::Work(key) => self.work(key).map(|w| w.date),
            NodeRef::Idea(id) => self.ideas.get(id).map(|i| i.date),
        }
    }

    #[must_use]
    pub fn resolves(&self, node: &NodeRef) -> bool {
        self.date_of(node).is_some()
    }
}
