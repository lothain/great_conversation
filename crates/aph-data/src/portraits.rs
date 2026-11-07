//! `portraits.json`: one licensed image per author, from Wikimedia Commons.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::PersonId;

pub type Portraits = BTreeMap<PersonId, Portrait>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Portrait {
    /// File name on Commons.
    pub file: String,
    /// Where the thumbnail was downloaded from.
    pub thumb_src: String,
    pub license: Option<String>,
    pub license_url: Option<String>,
    pub artist: Option<String>,
    pub credit: Option<String>,
    /// The Commons file page.
    pub commons: String,
    /// Local thumbnail, relative to the page (e.g. `portraits/mcluhan.jpg`).
    pub thumb: String,
    #[serde(default)]
    pub article: Option<String>,
    #[serde(default)]
    pub article_title: Option<String>,
}
