//! `aphorisms.json`: sections, authors, entries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    edtf::Edtf,
    geo::{Lat, Lon},
    ids::{EntryId, PersonId, SectionId, WorkKey},
    vocab::{Confidence, ReligionFamily},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    pub title: String,
    pub subtitle: String,
    pub date: String,
    pub author: String,
    pub sections: Vec<Section>,
    pub authors: BTreeMap<PersonId, Author>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: SectionId,
    pub name: String,
    pub intro: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Author {
    pub name: String,
    pub life: Edtf,
    /// Residence timeline, chronological.
    pub places: Vec<Residence>,
    pub religion: Vec<ReligionStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Residence {
    pub when: Edtf,
    pub name: String,
    pub lat: Lat,
    pub lon: Lon,
    pub conf: Confidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReligionStage {
    pub when: Edtf,
    pub family: ReligionFamily,
    pub detail: String,
    pub conf: Confidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: EntryId,
    pub section: SectionId,
    pub author: PersonId,
    pub work: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_id: Option<WorkKey>,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    /// When the words were written or spoken.
    pub date: Edtf,
    #[serde(default)]
    pub date_note: Option<String>,
    /// Where the author was at `date`.
    pub place: Place,
    pub context: String,
    pub quotes: Vec<Quote>,
    pub interpretation: String,
    pub source: Link,
    pub see_also: Vec<Link>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wikipedia: Option<Wikipedia>,
    pub needs: Vec<Need>,
    /// How the date, attribution, wording and source were settled: judgement calls,
    /// disputed attributions, secondary sources relied on, and what is still unchecked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_notes: Option<String>,
    pub rights: Rights,
    /// Who speaks the words when it isn't the author (e.g. the serpent in Genesis 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<Speaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speaker {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aka: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub name: String,
    pub lat: Lat,
    pub lon: Lon,
    pub conf: Confidence,
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    /// BCP-47 language tag of the original.
    pub lang: String,
    pub orig: String,
    pub translit: Option<String>,
    pub trans: Option<String>,
    pub trans_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub url: Option<String>,
}

/// An English Wikipedia article for the entry's work, or for its author when the work has none.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wikipedia {
    pub title: String,
    pub url: String,
    pub about: WikiSubject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WikiSubject {
    Work,
    Author,
}

/// Follow-up work an entry still needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Need {
    SourceUrl,
    VerifyQuote,
    VerifyOrig,
    VerifyAttribution,
    VerifyClaim,
    VerifyLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rights {
    pub status: RightsStatus,
    pub author_died: Option<i32>,
    pub translation_by: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RightsStatus {
    PublicDomain,
    PdCanadaOnly,
    InCopyright,
    NoQuote,
}

impl Quote {
    /// True when `orig` is written in Latin script and needs no transliteration.
    #[must_use]
    pub fn is_latin_script(&self) -> bool {
        self.orig
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| (c as u32) < 0x0370 || (0x1E00..0x1F00).contains(&(c as u32)))
    }
}
