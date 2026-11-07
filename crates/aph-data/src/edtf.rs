//! Extended Date/Time Format (ISO 8601-2), the subset this dataset uses:
//! `1957`, `1947-11-25`, `1939~`, `1939?`, `1939%`, `-0399/-0300`, `1960/..`.
//!
//! BCE years use astronomical numbering: 1 BCE = `0000`, 400 BCE = `-0399`.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// A calendar date at year, month or day precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

/// A totally ordered instant used to compare dates of mixed precision.
pub type Day = (i32, u8, u8);

impl Date {
    /// The first day this date could denote.
    #[must_use]
    pub fn earliest(self) -> Day {
        (self.year, self.month.unwrap_or(1), self.day.unwrap_or(1))
    }

    /// The last day this date could denote.
    #[must_use]
    pub fn latest(self) -> Day {
        (self.year, self.month.unwrap_or(12), self.day.unwrap_or(31))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Qualifier {
    #[default]
    Exact,
    /// `~`
    Approximate,
    /// `?`
    Uncertain,
    /// `%`
    ApproximateAndUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub date: Date,
    pub qualifier: Qualifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum End {
    At(Point),
    /// `..`: open or unknown end.
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Edtf {
    Point(Point),
    Interval { start: Point, end: End },
}

impl Edtf {
    #[must_use]
    pub fn earliest(self) -> Day {
        match self {
            Self::Point(p) | Self::Interval { start: p, .. } => p.date.earliest(),
        }
    }

    /// `None` for an open-ended interval.
    #[must_use]
    pub fn latest(self) -> Option<Day> {
        match self {
            Self::Point(p) | Self::Interval { end: End::At(p), .. } => Some(p.date.latest()),
            Self::Interval { end: End::Open, .. } => None,
        }
    }

    #[must_use]
    pub fn start_year(self) -> i32 {
        self.earliest().0
    }

    /// True when the two spans share at least one possible day.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        let before = |a: Self, b: Self| a.latest().is_some_and(|end| end < b.earliest());
        !before(self, other) && !before(other, self)
    }

    /// True when every possible day of `self` lies within `outer`.
    #[must_use]
    pub fn within(self, outer: Self) -> bool {
        outer.earliest() <= self.earliest()
            && match (outer.latest(), self.latest()) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(o), Some(s)) => s <= o,
            }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EdtfError {
    #[error("`{0}`: expected a 4-digit year, optionally signed")]
    Year(String),
    #[error("`{0}`: month must be 01-12")]
    Month(String),
    #[error("`{0}`: day must be 01-31")]
    Day(String),
    #[error("`{0}`: unexpected trailing text")]
    Trailing(String),
    #[error("`{0}`: interval end precedes its start")]
    Backwards(String),
}

fn two_digits(s: &str) -> Option<u8> {
    (s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok())?
}

impl FromStr for Point {
    type Err = EdtfError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (body, qualifier) = match s.as_bytes().last() {
            Some(b'~') => (&s[..s.len() - 1], Qualifier::Approximate),
            Some(b'?') => (&s[..s.len() - 1], Qualifier::Uncertain),
            Some(b'%') => (&s[..s.len() - 1], Qualifier::ApproximateAndUncertain),
            _ => (s, Qualifier::Exact),
        };
        let (negative, unsigned) = body.strip_prefix('-').map_or((false, body), |rest| (true, rest));
        let mut parts = unsigned.split('-');
        let year_text = parts.next().unwrap_or_default();
        if year_text.len() != 4 || !year_text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(EdtfError::Year(s.to_owned()));
        }
        let magnitude: i32 = year_text.parse().map_err(|_| EdtfError::Year(s.to_owned()))?;
        let year = if negative { -magnitude } else { magnitude };
        let month = parts
            .next()
            .map(|m| two_digits(m).filter(|m| (1..=12).contains(m)).ok_or_else(|| EdtfError::Month(s.to_owned())))
            .transpose()?;
        let day = parts
            .next()
            .map(|d| two_digits(d).filter(|d| (1..=31).contains(d)).ok_or_else(|| EdtfError::Day(s.to_owned())))
            .transpose()?;
        if parts.next().is_some() {
            return Err(EdtfError::Trailing(s.to_owned()));
        }
        Ok(Self { date: Date { year, month, day }, qualifier })
    }
}

impl FromStr for Edtf {
    type Err = EdtfError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((start, end)) = s.split_once('/') else {
            return s.parse().map(Self::Point);
        };
        let start: Point = start.parse()?;
        let end = if end == ".." { End::Open } else { End::At(end.parse()?) };
        let interval = Self::Interval { start, end };
        if interval.latest().is_some_and(|latest| latest < start.date.earliest()) {
            return Err(EdtfError::Backwards(s.to_owned()));
        }
        Ok(interval)
    }
}

impl TryFrom<String> for Edtf {
    type Error = EdtfError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Date { year, month, day } = self.date;
        if year < 0 {
            write!(f, "-{:04}", year.unsigned_abs())?;
        } else {
            write!(f, "{year:04}")?;
        }
        if let Some(m) = month {
            write!(f, "-{m:02}")?;
        }
        if let Some(d) = day {
            write!(f, "-{d:02}")?;
        }
        f.write_str(match self.qualifier {
            Qualifier::Exact => "",
            Qualifier::Approximate => "~",
            Qualifier::Uncertain => "?",
            Qualifier::ApproximateAndUncertain => "%",
        })
    }
}

impl fmt::Display for Edtf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Point(p) => p.fmt(f),
            Self::Interval { start, end: End::At(end) } => write!(f, "{start}/{end}"),
            Self::Interval { start, end: End::Open } => write!(f, "{start}/.."),
        }
    }
}

impl From<Edtf> for String {
    fn from(e: Edtf) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for s in ["1957", "1947-11-25", "1939~", "-0399/-0300", "1960/..", "0000", "1883-01/1883-02", "1930-11-17/1931"] {
            assert_eq!(s.parse::<Edtf>().unwrap().to_string(), s);
        }
    }

    #[test]
    fn rejects_malformed() {
        assert!("399 BCE".parse::<Edtf>().is_err());
        assert!("1939-13".parse::<Edtf>().is_err());
        assert!("1950/1940".parse::<Edtf>().is_err());
    }

    #[test]
    fn containment() {
        let e = |s: &str| s.parse::<Edtf>().unwrap();
        assert!(e("1921-05").within(e("1921/1921")));
        assert!(e("-0369~").within(e("-0386/-0368")));
        assert!(!e("1950").within(e("1933/1945")));
        assert!(e("2000").within(e("1960/..")));
        assert!(e("1939/1947").overlaps(e("1945")));
    }
}
