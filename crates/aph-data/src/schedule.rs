//! `curator_schedule.json`: when the curator's replies (each entry's `interpretation`) entered
//! the conversation, and the curator's own profile.
//!
//! Phases run in order; an author belongs to the first phase that lists them. Authors in no
//! phase fall into `rest`.

use serde::{Deserialize, Serialize};

use crate::{edtf::Edtf, ids::PersonId, vocab::ReligionFamily};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratorSchedule {
    #[serde(rename = "_doc", default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub curator: String,
    pub phases: Vec<Phase>,
    pub rest: Rest,
    pub profile: CuratorProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub name: String,
    /// EDTF months, e.g. `2005-01/2005-08`.
    pub when: Edtf,
    pub authors: Vec<PersonId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rest {
    pub name: String,
    pub when: Edtf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratorProfile {
    pub religion: Vec<CuratorFaith>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuratorFaith {
    pub when: Edtf,
    pub family: ReligionFamily,
    pub label: String,
    pub detail: String,
}

impl CuratorSchedule {
    /// Index of the phase an author's replies belong to; `phases.len()` means `rest`.
    #[must_use]
    pub fn phase_of(&self, author: &PersonId) -> usize {
        self.phases.iter().position(|p| p.authors.contains(author)).unwrap_or(self.phases.len())
    }

    /// (name, when) of phase `i`, where `phases.len()` is `rest`.
    #[must_use]
    pub fn phase(&self, i: usize) -> (&str, Edtf) {
        self.phases.get(i).map_or((self.rest.name.as_str(), self.rest.when), |p| (p.name.as_str(), p.when))
    }
}
