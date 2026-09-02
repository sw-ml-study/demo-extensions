//! Deterministic retained-playback state and public reducer.
use crate::{Frame, Recording};
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RunStatus {
    #[default]
    Idle,
    Ready,
    Done,
    Failed(String),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Playback {
    #[default]
    Paused,
    Playing,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPreference {
    #[default]
    Full,
    Reduced,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Load(Recording),
    Previous,
    Next,
    SeekIndex(usize),
    Select(usize),
    Play,
    Pause,
    Tick,
    Fail(String),
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewerState {
    pub(crate) recording: Option<Recording>,
    pub(crate) frame_index: usize,
    pub(crate) observation_index: usize,
    pub playback: Playback,
    pub motion: MotionPreference,
    pub run_status: RunStatus,
}
impl ViewerState {
    /// Apply one deterministic action.
    pub fn reduce(&mut self, action: Action) {
        match action {
            Action::Load(recording) => self.load(recording),
            Action::Previous => {
                crate::transitions::move_to(self, self.frame_index.saturating_sub(1));
            }
            Action::Next | Action::Tick => {
                crate::transitions::advance(self, matches!(action, Action::Tick));
            }
            Action::SeekIndex(index) => crate::transitions::seek(self, index),
            Action::Select(index) => crate::transitions::select(self, index),
            Action::Play => crate::transitions::play(self),
            Action::Pause => self.playback = Playback::Paused,
            Action::Fail(error) => crate::transitions::fail(self, error),
        }
    }
    /// Atomically replace the accepted recording.
    pub fn load(&mut self, recording: Recording) {
        self.recording = Some(recording);
        self.frame_index = 0;
        self.observation_index = 0;
        self.playback = Playback::Paused;
        self.run_status = RunStatus::Ready;
    }
    #[must_use]
    pub fn current_frame(&self) -> Option<&Frame> {
        self.recording.as_ref()?.frames.get(self.frame_index)
    }
    #[must_use]
    pub const fn frame_index(&self) -> usize {
        self.frame_index
    }
    #[must_use]
    pub const fn observation_index(&self) -> usize {
        self.observation_index
    }
    #[must_use]
    pub const fn recording(&self) -> Option<&Recording> {
        self.recording.as_ref()
    }
    #[must_use]
    pub fn current_observation(&self) -> Option<&crate::Observation> {
        self.current_frame()?
            .observations
            .get(self.observation_index)
    }
}
