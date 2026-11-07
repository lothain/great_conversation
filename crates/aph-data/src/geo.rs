//! Range-checked coordinates.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
#[error("{axis} {value} out of range")]
pub struct OutOfRange {
    axis: &'static str,
    value: f64,
}

macro_rules! bounded {
    ($name:ident, $axis:literal, $limit:literal) => {
        #[doc = concat!("Decimal-degree ", $axis, ", within ±", stringify!($limit), ".")]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(try_from = "f64", into = "f64")]
        pub struct $name(f64);

        impl TryFrom<f64> for $name {
            type Error = OutOfRange;
            fn try_from(value: f64) -> Result<Self, Self::Error> {
                if value.is_finite() && (-$limit..=$limit).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(OutOfRange { axis: $axis, value })
                }
            }
        }

        impl From<$name> for f64 {
            fn from(v: $name) -> f64 {
                v.0
            }
        }
    };
}

bounded!(Lat, "latitude", 90.0);
bounded!(Lon, "longitude", 180.0);

/// A `GeoJSON` position, `[lon, lat]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 2]", into = "[f64; 2]")]
pub struct Position {
    pub lon: Lon,
    pub lat: Lat,
}

impl TryFrom<[f64; 2]> for Position {
    type Error = OutOfRange;
    fn try_from([lon, lat]: [f64; 2]) -> Result<Self, Self::Error> {
        Ok(Self { lon: lon.try_into()?, lat: lat.try_into()? })
    }
}

impl From<Position> for [f64; 2] {
    fn from(p: Position) -> Self {
        [p.lon.into(), p.lat.into()]
    }
}

impl Position {
    #[must_use]
    pub fn xy(self) -> (f64, f64) {
        (self.lon.into(), self.lat.into())
    }
}
