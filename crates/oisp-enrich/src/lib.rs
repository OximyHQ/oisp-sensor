//! Enrichment plugins for OISP Sensor

pub mod host;
pub mod process_tree;

pub use host::HostEnricher;
pub use process_tree::ProcessTreeEnricher;
