//! Simple implementations of the fallback principle, allocating from one and fallbacking to another.
//!
//! This is an attempt at implementing a generic way to combining existing storages, to simplify implementating small
//! storages for example.
//!
//! It is simpler than alternative, however is heavier weight.

mod fallback_element;
mod fallback_range;

pub use fallback_element::FallbackElement;
pub use fallback_range::FallbackRange;
