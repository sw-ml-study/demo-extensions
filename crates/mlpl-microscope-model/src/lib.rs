//! Pure recording, reducer, and view-planning model for the microscope viewer.

mod protocol;
mod reducer;
mod transitions;
mod validation;
mod validation_support;
mod view;

pub use protocol::{Budgets, Frame, Observation, Recording};
pub use reducer::{Action, MotionPreference, Playback, RunStatus, ViewerState};
pub use validation::{ValidationError, ValidationKind, parse_recording};
pub use view::{Summary, ViewKind, ViewPlan, repeated_series};
