//! Enrichment plugins for OISP Sensor
//!
//! Built-in enrichers that add context to events.

mod host;
mod process_tree;

pub use host::HostEnricher;
pub use process_tree::ProcessTreeEnricher;

