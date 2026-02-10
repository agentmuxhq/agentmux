// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! OType constants and ORef (typed object reference).

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use uuid::Uuid;

// ---- OType constants (match Go's waveobj.OType_* constants) ----

pub const OTYPE_CLIENT: &str = "client";
pub const OTYPE_WINDOW: &str = "window";
pub const OTYPE_WORKSPACE: &str = "workspace";
pub const OTYPE_TAB: &str = "tab";
pub const OTYPE_LAYOUT: &str = "layout";
pub const OTYPE_BLOCK: &str = "block";
pub const OTYPE_TEMP: &str = "temp";

pub const VALID_OTYPES: &[&str] = &[
    OTYPE_CLIENT,
    OTYPE_WINDOW,
    OTYPE_WORKSPACE,
    OTYPE_TAB,
    OTYPE_LAYOUT,
    OTYPE_BLOCK,
    OTYPE_TEMP,
];

// ---- ORef ----

/// Object reference combining a type name and UUID.
/// Wire format: `"block:550e8400-e29b-41d4-a716-446655440000"`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ORef {
    pub otype: String,
    pub oid: String,
}

impl ORef {
    pub fn new(otype: impl Into<String>, oid: impl Into<String>) -> Self {
        Self {
            otype: otype.into(),
            oid: oid.into(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.otype.is_empty() || self.oid.is_empty()
    }

    /// Parse an ORef from the "otype:oid" string format.
    pub fn parse(s: &str) -> Result<Self, ORefParseError> {
        if s.is_empty() {
            return Ok(Self {
                otype: String::new(),
                oid: String::new(),
            });
        }

        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ORefParseError::InvalidFormat(s.to_string()));
        }

        let otype = parts[0];
        let oid = parts[1];

        if otype.is_empty() || !otype.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(ORefParseError::InvalidOType(otype.to_string()));
        }

        if !VALID_OTYPES.contains(&otype) {
            return Err(ORefParseError::UnknownOType(otype.to_string()));
        }

        Uuid::parse_str(oid).map_err(|_| ORefParseError::InvalidOID(oid.to_string()))?;

        Ok(Self {
            otype: otype.to_string(),
            oid: oid.to_string(),
        })
    }
}

impl fmt::Display for ORef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "");
        }
        write!(f, "{}:{}", self.otype, self.oid)
    }
}

/// Errors from parsing an ORef string.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ORefParseError {
    #[error("invalid object reference: {0:?}")]
    InvalidFormat(String),
    #[error("invalid object type: {0:?}")]
    InvalidOType(String),
    #[error("unknown object type: {0:?}")]
    UnknownOType(String),
    #[error("invalid object id: {0:?}")]
    InvalidOID(String),
}

impl Serialize for ORef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ORef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ORef::parse(&s).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oref_roundtrip() {
        let oref = ORef::new("block", "550e8400-e29b-41d4-a716-446655440000");
        let json = serde_json::to_string(&oref).unwrap();
        assert_eq!(json, r#""block:550e8400-e29b-41d4-a716-446655440000""#);
        let parsed: ORef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, oref);
    }

    #[test]
    fn test_oref_empty() {
        let oref = ORef::parse("").unwrap();
        assert!(oref.is_empty());
    }

    #[test]
    fn test_oref_all_valid_types() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        for otype in VALID_OTYPES {
            let s = format!("{otype}:{uuid}");
            let oref = ORef::parse(&s).unwrap();
            assert_eq!(oref.otype, *otype);
        }
    }

    #[test]
    fn test_oref_invalid_format() {
        assert!(ORef::parse("blockuuid").is_err());
    }

    #[test]
    fn test_oref_unknown_otype() {
        assert!(ORef::parse("foobar:550e8400-e29b-41d4-a716-446655440000").is_err());
    }

    #[test]
    fn test_oref_invalid_uuid() {
        assert!(ORef::parse("block:not-a-uuid").is_err());
    }
}
