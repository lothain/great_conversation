//! Identifier newtypes, so an author id can't be passed where an entry id is expected.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

macro_rules! id {
    ($($name:ident),* $(,)?) => {$(
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }
    )*};
}

id!(PersonId, SectionId, EntryId, WorkSlug, IdeaId, CivId);

/// `person/slug`: a work owned by a person, e.g. `kant/prolegomena`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkKey {
    pub person: PersonId,
    pub slug: WorkSlug,
}

#[derive(Debug, thiserror::Error)]
#[error("`{0}`: expected `person/work`")]
pub struct BadWorkKey(String);

impl FromStr for WorkKey {
    type Err = BadWorkKey;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('/') {
            Some((p, w)) if !p.is_empty() && !w.is_empty() && !w.contains('/') => {
                Ok(Self { person: PersonId(p.to_owned()), slug: WorkSlug(w.to_owned()) })
            }
            _ => Err(BadWorkKey(s.to_owned())),
        }
    }
}

impl TryFrom<String> for WorkKey {
    type Error = BadWorkKey;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl fmt::Display for WorkKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.person, self.slug)
    }
}

impl From<WorkKey> for String {
    fn from(k: WorkKey) -> Self {
        k.to_string()
    }
}
