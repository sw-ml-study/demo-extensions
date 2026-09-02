//! Browser-independent SSE ingestion used by the Yew microscope.

mod assembly;
mod sse;

pub use assembly::{LiveAssembler, LiveEvent, LivePhase};
pub use sse::{SseEvent, SseParser};
