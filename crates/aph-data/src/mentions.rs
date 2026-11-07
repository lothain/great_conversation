//! `mentions.json`: one utterance calling back to another (an "@mention").
//!
//! Lighter than a `graph.json` response: utterance to utterance (entry ids, not works), and
//! allowed to be an echo, allusion or shared formula with no claim of direct influence.

use serde::{Deserialize, Serialize};

use crate::{graph::Basis, ids::EntryId, vocab::Confidence};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mentions {
    #[serde(rename = "_doc", default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub mentions: Vec<Mention>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mention {
    pub from: EntryId,
    pub to: EntryId,
    pub kind: MentionKind,
    /// The words that recur, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phrase: Option<String>,
    /// What is echoed, ≤ 25 words.
    pub note: String,
    pub basis: Basis,
    pub conf: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MentionKind {
    Callback,
    Echo,
    Allusion,
    Formula,
    Quotes,
}
