// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Domain layer: Pure domain models with zero dependencies on Tauri or storage.
//!
//! This module contains:
//! - **entities/**: Core WaveObj types (Client, Window, Workspace, Tab, Block, Layout)
//! - **value_objects/**: Typed IDs, MetaMap, geometric types
//! - **traits/**: Repository contracts for storage implementations
//! - **events/**: Domain event types

pub mod entities;
pub mod events;
pub mod traits;
pub mod value_objects;
