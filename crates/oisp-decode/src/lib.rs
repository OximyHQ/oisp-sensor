//! Event decoders for OISP Sensor
//!
//! This crate provides decoders that transform raw capture events into
//! structured OISP events:
//!
//! - **HttpDecoder**: Decodes SSL/TLS traffic into HTTP and AI events
//! - **SystemDecoder**: Decodes process, file, and network events

pub mod ai;
pub mod decoder;
pub mod http;
pub mod spec_parser;
pub mod sse;
pub mod system;

pub use decoder::HttpDecoder;
pub use spec_parser::SpecDrivenParser;
pub use system::SystemDecoder;
