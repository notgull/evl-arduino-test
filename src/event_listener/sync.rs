//! Implementation of synchronization primitives.

// TODO: portable_atomic or loom implementations

pub use core::cell;
pub use portable_atomic as atomic;
