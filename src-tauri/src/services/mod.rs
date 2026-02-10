// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Service layer: business logic encapsulated in stateless services.
//!
//! Services depend **only** on domain traits and entities — never on
//! Tauri, storage implementations, or presentation concerns.

pub mod workspace_service;
pub mod tab_service;
pub mod object_service;
