#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerButtons(u8);

impl PointerButtons {
    pub const NONE: Self = Self(0);
    pub const LEFT: Self = Self(1);
    pub const MIDDLE: Self = Self(2);
    pub const RIGHT: Self = Self(4);
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn with(self, button: Self, pressed: bool) -> Self {
        if pressed {
            Self(self.0 | button.0)
        } else {
            Self(self.0 & !button.0)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(1);
    pub const CONTROL: Self = Self(2);
    pub const ALT: Self = Self(4);
    pub const META: Self = Self(8);
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn from_flags([shift, control, alt, meta]: [bool; 4]) -> Self {
        Self((shift as u8) | ((control as u8) << 1) | ((alt as u8) << 2) | ((meta as u8) << 3))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    PointerMove {
        position: [f64; 2],
        buttons: PointerButtons,
        modifiers: Modifiers,
    },
    PointerButton {
        button: PointerButton,
        pressed: bool,
        position: [f64; 2],
        modifiers: Modifiers,
    },
    Wheel {
        delta: [f64; 2],
        position: [f64; 2],
        modifiers: Modifiers,
    },
    Frame {
        delta_ms: f64,
        elapsed_ms: f64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    InvalidCapacity,
    NonFinite,
    Full,
}

pub struct BoundedInput {
    capacity: usize,
    events: Vec<InputEvent>,
}

impl InputEvent {
    #[must_use]
    pub const fn pointer_move(
        position: [f64; 2],
        buttons: PointerButtons,
        modifiers: Modifiers,
    ) -> Self {
        Self::PointerMove {
            position,
            buttons,
            modifiers,
        }
    }
    #[must_use]
    pub const fn pointer_button(
        button: PointerButton,
        pressed: bool,
        position: [f64; 2],
        modifiers: Modifiers,
    ) -> Self {
        Self::PointerButton {
            button,
            pressed,
            position,
            modifiers,
        }
    }
    #[must_use]
    pub const fn wheel(delta: [f64; 2], position: [f64; 2], modifiers: Modifiers) -> Self {
        Self::Wheel {
            delta,
            position,
            modifiers,
        }
    }
    #[must_use]
    pub const fn frame(delta_ms: f64, elapsed_ms: f64) -> Self {
        Self::Frame {
            delta_ms,
            elapsed_ms,
        }
    }
    fn finite(self) -> bool {
        match self {
            Self::PointerMove { position, .. } | Self::PointerButton { position, .. } => {
                position.into_iter().all(f64::is_finite)
            }
            Self::Wheel {
                delta, position, ..
            } => delta.into_iter().chain(position).all(f64::is_finite),
            Self::Frame {
                delta_ms,
                elapsed_ms,
            } => {
                delta_ms.is_finite()
                    && elapsed_ms.is_finite()
                    && delta_ms >= 0.0
                    && elapsed_ms >= 0.0
            }
        }
    }
}

impl BoundedInput {
    /// Creates a fixed-capacity pending input buffer.
    ///
    /// # Errors
    ///
    /// Rejects zero capacity.
    pub fn new(capacity: usize) -> Result<Self, InputError> {
        if capacity == 0 {
            return Err(InputError::InvalidCapacity);
        }
        Ok(Self {
            capacity,
            events: Vec::with_capacity(capacity),
        })
    }

    /// Validates and queues or coalesces an input event.
    ///
    /// # Errors
    ///
    /// Rejects non-finite input or a full buffer when the event cannot be
    /// coalesced with a pending event of the same high-rate kind.
    pub fn push(&mut self, event: InputEvent) -> Result<(), InputError> {
        if !event.finite() {
            return Err(InputError::NonFinite);
        }
        if let Some(index) = self.coalescing_index(event) {
            self.events[index] = match (self.events[index], event) {
                (
                    InputEvent::Wheel { delta: old, .. },
                    InputEvent::Wheel {
                        delta,
                        position,
                        modifiers,
                    },
                ) => InputEvent::wheel([old[0] + delta[0], old[1] + delta[1]], position, modifiers),
                (_, replacement) => replacement,
            };
            return Ok(());
        }
        if self.events.len() == self.capacity {
            return Err(InputError::Full);
        }
        self.events.push(event);
        Ok(())
    }

    #[must_use]
    pub fn drain(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }

    fn coalescing_index(&self, event: InputEvent) -> Option<usize> {
        let same_kind = |candidate: &InputEvent| {
            matches!(
                (candidate, event),
                (
                    InputEvent::PointerMove { .. },
                    InputEvent::PointerMove { .. }
                ) | (InputEvent::Wheel { .. }, InputEvent::Wheel { .. })
                    | (InputEvent::Frame { .. }, InputEvent::Frame { .. })
            )
        };
        self.events.iter().rposition(same_kind)
    }
}
