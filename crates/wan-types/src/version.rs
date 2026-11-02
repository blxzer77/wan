use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use specta::Type;

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Type)]
#[specta(type = String)]
pub struct ProtocolVersion {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ProtocolVersionError {
    InvalidFormat,
    InvalidDate,
}

impl ProtocolVersion {
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, ProtocolVersionError> {
        if !valid_date(year, month, day) {
            return Err(ProtocolVersionError::InvalidDate);
        }
        Ok(Self { year, month, day })
    }

    pub fn parse(value: &str) -> Result<Self, ProtocolVersionError> {
        let bytes = value.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(ProtocolVersionError::InvalidFormat);
        }

        let year = parse_component(&bytes[0..4]).ok_or(ProtocolVersionError::InvalidFormat)?;
        let month = parse_component(&bytes[5..7]).ok_or(ProtocolVersionError::InvalidFormat)?;
        let day = parse_component(&bytes[8..10]).ok_or(ProtocolVersionError::InvalidFormat)?;
        Self::new(year, month as u8, day as u8)
    }

    pub const fn year(self) -> u16 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }

    pub const fn is_compatible_with(self, other: Self) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

impl Serialize for ProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ProtocolVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => formatter.write_str("protocol version must use YYYY-MM-DD"),
            Self::InvalidDate => formatter.write_str("protocol version contains an invalid date"),
        }
    }
}

impl std::error::Error for ProtocolVersionError {}

fn parse_component(bytes: &[u8]) -> Option<u16> {
    if bytes.iter().any(|byte| !byte.is_ascii_digit()) {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0, |value, byte| value * 10 + u16::from(byte - b'0')),
    )
}

const fn valid_date(year: u16, month: u8, day: u8) -> bool {
    if month == 0 || month > 12 || day == 0 {
        return false;
    }
    day <= days_in_month(year, month)
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

const fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

#[cfg(test)]
mod tests {
    use super::{ProtocolVersion, ProtocolVersionError};

    #[test]
    fn parses_and_formats_date_versions() {
        let version = ProtocolVersion::parse("2026-11-02").expect("valid date");
        assert_eq!(version.year(), 2026);
        assert_eq!(version.month(), 11);
        assert_eq!(version.day(), 2);
        assert_eq!(version.to_string(), "2026-11-02");
    }

    #[test]
    fn rejects_invalid_date_versions() {
        assert_eq!(
            ProtocolVersion::parse("2026/11/02"),
            Err(ProtocolVersionError::InvalidFormat)
        );
        assert_eq!(
            ProtocolVersion::parse("2026-02-30"),
            Err(ProtocolVersionError::InvalidDate)
        );
    }

    #[test]
    fn serializes_as_a_date_string() {
        let version = ProtocolVersion::parse("2026-11-02").unwrap();
        let json = serde_json::to_string(&version).unwrap();
        assert_eq!(json, "\"2026-11-02\"");
        assert_eq!(
            serde_json::from_str::<ProtocolVersion>(&json).unwrap(),
            version
        );
    }

    #[test]
    fn only_equal_dates_are_compatible() {
        let current = ProtocolVersion::parse("2026-11-02").unwrap();
        assert!(current.is_compatible_with(current));
        assert!(!current.is_compatible_with(ProtocolVersion::parse("2026-11-03").unwrap()));
    }
}
