//! Target v1 domain model.
//!
//! This module is intentionally separate from `crate::models`, which still
//! describes the current JSON implementation. New data-platform code
//! must use these types so the two schemas cannot be mixed accidentally.

mod achievements;
mod derived_values;
mod expression;
mod ids;
mod memory;
mod missions;
mod packs;
mod records;
mod repository;
mod skills;
mod status;
mod validation;

pub use achievements::*;
pub use derived_values::*;
pub use expression::*;
pub use ids::*;
pub use memory::*;
pub use missions::*;
pub use packs::*;
pub use records::*;
pub use repository::*;
pub use skills::*;
pub use status::*;
pub use validation::*;

/// Repository schema. Version 2 shards AchievementState JSON by Pack.
/// The legacy alias remains for existing callers.
pub const SCHEMA_VERSION: u32 = 2;
pub const REPOSITORY_SCHEMA_VERSION: u32 = SCHEMA_VERSION;
/// Pack schema v1 remains readable; new Packs use v2 with DerivedValues.
pub const LEGACY_PACK_SCHEMA_VERSION: u32 = 1;
pub const PACK_SCHEMA_VERSION: u32 = 2;
