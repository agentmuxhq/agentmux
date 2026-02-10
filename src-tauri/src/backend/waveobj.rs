// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! WaveObj types — re-exported from the domain layer for backwards compatibility.
//!
//! New code should import from `crate::domain::entities` and `crate::domain::value_objects`.

// Re-export all entities (Client, Window, Workspace, Tab, Block, LayoutState, etc.)
pub use crate::domain::entities::*;

// Re-export all value objects (MetaMapType, merge_meta, meta_get_*, ORef, Point, etc.)
pub use crate::domain::value_objects::*;
