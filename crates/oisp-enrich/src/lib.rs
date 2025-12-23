//! Enrichment plugins for OISP Sensor

pub mod process_tree;
pub mod host;

pub use process_tree::ProcessTreeEnricher;
pub use host::HostEnricher;

