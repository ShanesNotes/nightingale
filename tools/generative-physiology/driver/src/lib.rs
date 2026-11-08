//! The teacher driver: one teacher run per process from a run spec, through the engine's public items only.
//! A run's directory is written whole or not at all.

mod capture;
mod link_file;
mod npy;
mod spec;

pub use capture::{BeatReading, Catalogue, CatalogueError, digest_of_words, step_row, words};
pub use link_file::LinkFile;
pub use npy::{Element, NpyWriter, Table};
pub use spec::{
    CAPTURE_VERSION, CoreChoice, PostureEvent, RunSpec, Setting, SpecError, SpecId, Stamp, Trunk, Variant,
    digest_of_hex, hex,
};
