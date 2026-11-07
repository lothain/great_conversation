//! Controlled vocabularies shared across datasets.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Med,
    Low,
}

/// Religion families, used for both authors and civilizations so they share one color scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReligionFamily {
    Judaism,
    Christianity,
    Islam,
    Hinduism,
    Buddhism,
    Jainism,
    Zoroastrianism,
    Manichaeism,
    #[serde(rename = "Greek polytheism")]
    GreekPolytheism,
    #[serde(rename = "Roman polytheism")]
    RomanPolytheism,
    #[serde(rename = "Germanic paganism")]
    GermanicPaganism,
    #[serde(rename = "Egyptian religion")]
    EgyptianReligion,
    #[serde(rename = "Mesopotamian religion")]
    MesopotamianReligion,
    #[serde(rename = "Chinese folk religion")]
    ChineseFolkReligion,
    Confucianism,
    Taoism,
    #[serde(rename = "Indigenous religions")]
    IndigenousReligions,
    Gnosticism,
    Deism,
    Pantheism,
    Agnosticism,
    Atheism,
    #[serde(rename = "Secular/nonreligious")]
    Secular,
    Other,
    Unknown,
}

impl ReligionFamily {
    pub const ALL: [Self; 25] = [
        Self::Judaism, Self::Christianity, Self::Islam, Self::Hinduism, Self::Buddhism, Self::Jainism,
        Self::Zoroastrianism, Self::Manichaeism, Self::GreekPolytheism, Self::RomanPolytheism,
        Self::GermanicPaganism, Self::EgyptianReligion, Self::MesopotamianReligion, Self::ChineseFolkReligion,
        Self::Confucianism, Self::Taoism, Self::IndigenousReligions, Self::Gnosticism, Self::Deism,
        Self::Pantheism, Self::Agnosticism, Self::Atheism, Self::Secular, Self::Other, Self::Unknown,
    ];

    /// Position in [`Self::ALL`], used as a bit index in shader masks.
    #[must_use]
    pub fn index(self) -> u32 {
        Self::ALL.iter().position(|f| *f == self).expect("ALL lists every family") as u32
    }
}
