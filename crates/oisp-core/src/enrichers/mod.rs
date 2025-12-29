//! Enrichment plugins for OISP Sensor
//!
//! Built-in enrichers that add context to events.

mod app;
mod host;
mod process_tree;

pub use app::AppEnricher;
pub use host::HostEnricher;
pub use process_tree::ProcessTreeEnricher;
