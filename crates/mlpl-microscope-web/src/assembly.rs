//! Ordered live frame assembly independent of browser networking.

use mlpl_microscope_model::{Budgets, Frame, Observation, Recording};
use serde::Deserialize;

/// Current stream phase.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LivePhase {
    #[default]
    Waiting,
    Ready,
    Done,
    Failed(String),
}

/// Typed event accepted from the SSE parser.
#[derive(Clone, Debug, PartialEq)]
pub enum LiveEvent {
    Ready,
    Frame { step: u64, observation: Observation },
    Done,
    Error(String),
}

/// Bounded in-order assembler for one live run.
#[derive(Clone, Debug)]
pub struct LiveAssembler {
    lesson: String,
    budgets: Budgets,
    frames: Vec<Frame>,
    total_values: u64,
    pub phase: LivePhase,
}

impl LiveAssembler {
    /// Create an empty assembler with consumer-chosen hard limits.
    #[must_use]
    pub fn new(lesson: impl Into<String>, budgets: Budgets) -> Self {
        Self {
            lesson: lesson.into(),
            budgets,
            frames: Vec::new(),
            total_values: 0,
            phase: LivePhase::Waiting,
        }
    }

    /// Decode a named SSE event without trusting producer markup.
    ///
    /// # Errors
    /// Returns an error for unknown event names or malformed frame JSON.
    pub fn decode(event: &crate::SseEvent) -> Result<LiveEvent, String> {
        match event.event.as_str() {
            "ready" => Ok(LiveEvent::Ready),
            "frame" => serde_json::from_str::<WireFrame>(&event.data)
                .map(|wire| LiveEvent::Frame {
                    step: wire.step,
                    observation: Observation {
                        name: wire.name,
                        shape: wire.shape,
                        values: wire.values,
                    },
                })
                .map_err(|error| error.to_string()),
            "done" => Ok(LiveEvent::Done),
            "error" => Ok(LiveEvent::Error(event.data.clone())),
            other => Err(format!("unsupported SSE event {other}")),
        }
    }

    /// Apply one event while retaining no partial over-budget frame.
    ///
    /// # Errors
    /// Returns an error when a frame is invalid, out of order, or over budget.
    pub fn apply(&mut self, event: LiveEvent) -> Result<(), String> {
        match event {
            LiveEvent::Ready => self.phase = LivePhase::Ready,
            LiveEvent::Frame { step, observation } => self.push_frame(step, observation)?,
            LiveEvent::Done => self.phase = LivePhase::Done,
            LiveEvent::Error(error) => self.phase = LivePhase::Failed(error),
        }
        Ok(())
    }

    /// Convert a terminal successful stream into the shared validated model.
    ///
    /// # Errors
    /// Returns an error before `done` or when final model validation fails.
    pub fn recording(&self) -> Result<Recording, String> {
        if self.phase != LivePhase::Done {
            return Err("live stream is not done".into());
        }
        let json = serde_json::json!({"schema":"sw-ml-study.ml-microscope-recording","version":0,"lesson":self.lesson,"budgets":self.budgets,"frames":self.frames});
        mlpl_microscope_model::parse_recording(&json.to_string()).map_err(|error| error.to_string())
    }

    fn push_frame(&mut self, step: u64, observation: Observation) -> Result<(), String> {
        if !observation.values.iter().all(|value| value.is_finite()) {
            return Err("frame values must be finite".into());
        }
        let value_count =
            u64::try_from(observation.values.len()).map_err(|_| "value count overflow")?;
        let next_total = self
            .total_values
            .checked_add(value_count)
            .ok_or("total value overflow")?;
        if value_count > self.budgets.max_values_per_observation
            || next_total > self.budgets.max_total_values
        {
            return Err("live value budget exceeded".into());
        }
        if let Some(frame) = self.frames.last_mut().filter(|frame| frame.step == step) {
            if u64::try_from(frame.observations.len()).unwrap_or(u64::MAX)
                >= self.budgets.max_observations_per_frame
            {
                return Err("live observation budget exceeded".into());
            }
            frame.observations.push(observation);
        } else {
            if self.frames.last().is_some_and(|frame| frame.step >= step) {
                return Err("live steps must increase".into());
            }
            if u64::try_from(self.frames.len()).unwrap_or(u64::MAX) >= self.budgets.max_frames {
                return Err("live frame budget exceeded".into());
            }
            self.frames.push(Frame {
                step,
                observations: vec![observation],
            });
        }
        self.total_values = next_total;
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireFrame {
    name: String,
    step: u64,
    shape: Vec<u64>,
    values: Vec<f64>,
}
