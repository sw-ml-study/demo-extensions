use std::any::Any;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeHandle {
    extension_id: u64,
    type_id: u64,
    slot: u32,
    generation: u32,
}

impl NativeHandle {
    #[must_use]
    pub const fn from_parts(extension_id: u64, type_id: u64, slot: u32, generation: u32) -> Self {
        Self {
            extension_id,
            type_id,
            slot,
            generation,
        }
    }
    #[must_use]
    pub const fn extension_id(self) -> u64 {
        self.extension_id
    }
    #[must_use]
    pub const fn type_id(self) -> u64 {
        self.type_id
    }
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleError {
    Inactive,
    Exhausted,
    WrongExtension,
    WrongType,
    Stale,
}

struct Entry {
    type_id: u64,
    value: Box<dyn Any>,
}

struct Slot {
    generation: u32,
    retired: bool,
    entry: Option<Entry>,
}

pub struct HandleRegistry {
    extension_id: u64,
    max_slots: usize,
    generation_limit: u32,
    slots: Vec<Slot>,
    active: bool,
}

impl HandleRegistry {
    #[must_use]
    pub const fn with_limits(extension_id: u64, max_slots: usize, generation_limit: u32) -> Self {
        Self {
            extension_id,
            max_slots,
            generation_limit,
            slots: Vec::new(),
            active: true,
        }
    }

    /// Stores one extension-owned resource and returns an opaque capability.
    ///
    /// # Errors
    ///
    /// Rejects inactive registries and exhausted slot/generation capacity.
    pub fn insert<T: 'static>(
        &mut self,
        type_id: u64,
        value: T,
    ) -> Result<NativeHandle, HandleError> {
        if !self.active {
            return Err(HandleError::Inactive);
        }
        let index = if let Some(index) = self
            .slots
            .iter()
            .position(|slot| !slot.retired && slot.entry.is_none())
        {
            index
        } else {
            if self.slots.len() >= self.max_slots || self.slots.len() > u32::MAX as usize {
                return Err(HandleError::Exhausted);
            }
            self.slots.push(Slot {
                generation: 1,
                retired: false,
                entry: None,
            });
            self.slots.len() - 1
        };
        let slot_id = u32::try_from(index).map_err(|_| HandleError::Exhausted)?;
        let slot = &mut self.slots[index];
        slot.entry = Some(Entry {
            type_id,
            value: Box::new(value),
        });
        Ok(NativeHandle::from_parts(
            self.extension_id,
            type_id,
            slot_id,
            slot.generation,
        ))
    }

    /// Borrows a resource after validating its full capability identity.
    ///
    /// # Errors
    ///
    /// Rejects inactive, foreign, wrong-type, and stale handles.
    pub fn get<T: 'static>(&self, handle: NativeHandle, type_id: u64) -> Result<&T, HandleError> {
        let entry = self.validate(handle, type_id)?;
        entry.value.downcast_ref().ok_or(HandleError::WrongType)
    }

    /// Removes and returns a resource, invalidating its generation first.
    ///
    /// # Errors
    ///
    /// Rejects inactive, foreign, wrong-type, and stale handles.
    pub fn remove<T: 'static>(
        &mut self,
        handle: NativeHandle,
        type_id: u64,
    ) -> Result<T, HandleError> {
        self.validate(handle, type_id)?
            .value
            .downcast_ref::<T>()
            .ok_or(HandleError::WrongType)?;
        let slot = &mut self.slots[handle.slot as usize];
        let entry = slot.entry.take().ok_or(HandleError::Stale)?;
        if slot.generation >= self.generation_limit {
            slot.retired = true;
        } else {
            slot.generation += 1;
        }
        entry
            .value
            .downcast::<T>()
            .map(|value| *value)
            .map_err(|_| HandleError::WrongType)
    }

    pub fn deactivate(&mut self) {
        if self.active {
            self.active = false;
            for slot in &mut self.slots {
                drop(slot.entry.take());
            }
        }
    }

    fn validate(&self, handle: NativeHandle, type_id: u64) -> Result<&Entry, HandleError> {
        if !self.active {
            return Err(HandleError::Inactive);
        }
        if handle.extension_id != self.extension_id {
            return Err(HandleError::WrongExtension);
        }
        if handle.type_id != type_id {
            return Err(HandleError::WrongType);
        }
        let slot = self
            .slots
            .get(handle.slot as usize)
            .ok_or(HandleError::Stale)?;
        if slot.retired || slot.generation != handle.generation {
            return Err(HandleError::Stale);
        }
        let entry = slot.entry.as_ref().ok_or(HandleError::Stale)?;
        if entry.type_id != type_id {
            return Err(HandleError::WrongType);
        }
        Ok(entry)
    }
}

impl Drop for HandleRegistry {
    fn drop(&mut self) {
        self.deactivate();
    }
}
