// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! ORef: re-exported from domain layer for backwards compatibility.
//!
//! New code should import from `crate::domain::value_objects`.

pub use crate::domain::value_objects::ids::{ORef, ORefParseError, VALID_OTYPES};
