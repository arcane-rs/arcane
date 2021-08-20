//! Re-exports of [`arcana_codegen`].

pub use arcana_codegen::{sa, unique_events};

/// TODO:
pub trait UniqueArcanaEvent {
    const SIZE: usize;
}
