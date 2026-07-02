pub mod achievements;
pub mod collections;
pub mod conflict;
pub mod conflict_assessment;
pub mod dependency;
pub mod detector;
pub mod errors;
pub mod fsutil;
pub mod launch;
pub mod locations;
pub mod models;
pub mod parser;
pub mod settings;

// Re-export the types that make up the crate's primary public surface.
pub use models::{DetectedGame, ModCollection, ModConflict, ModDescriptor};
pub use settings::UserSettings;
