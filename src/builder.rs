//! Typestate markers shared by the crate's builders.
//!
//! A typestate builder carries a required field as a type parameter that starts
//! as [`Missing`] and becomes the field's value once it is set. The terminal
//! (`build()`) exists only on the complete state, so a builder missing a required
//! field fails to compile instead of failing at runtime.

/// Marker for a required field a typestate builder has not been given yet.
#[derive(Debug, Clone, Copy)]
pub struct Missing;

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
