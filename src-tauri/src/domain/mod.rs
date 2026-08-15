//! Target v1 domain model.
//!
//! This module is intentionally separate from `crate::models`, which still
//! describes the current JSON implementation. New data-platform code
//! must use these types so the two schemas cannot be mixed accidentally.

mod achievements;
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

/// Repository and pack schema supported by the first target implementation.
pub const SCHEMA_VERSION: u32 = 1;
