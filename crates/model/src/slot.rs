use crate::blocks::Block;

use std::sync::RwLock;

use std::sync::Arc;

use std::fmt::Debug;

use anyhow::anyhow;

/// Shared, mutable slot for a Block. This allows us to calculate complex recursive relationships between Blocks inside
/// the Block struct itself.
///
/// It acts as a deferred initialization so that we can construct recursive datastructures without infinite recursion.
///
/// Once the struct using this is constructed to a usable state, all atomic block slots should be *Set*. Reading
/// an atomic slot returns a result to reflect this.
#[derive(Clone)]
pub struct Slot<T: Clone>(Arc<RwLock<SlotState<T>>>);

#[derive(Clone)]
pub(crate) enum SlotState<T> {
    Empty,
    Set(T),
}

impl<T: Clone> Slot<T> {
    pub(crate) fn empty() -> Self {
        Self(Arc::new(RwLock::new(SlotState::Empty)))
    }

    pub(crate) fn set(&self, item: T) -> anyhow::Result<Self> {
        let mut write = self
            .0
            .write()
            .or(Err(anyhow!("Failed to write lock when it should have")))?;
        *write = SlotState::Set(item);

        Ok(self.clone())
    }

    pub(crate) fn new(item: T) -> Self {
        Self(Arc::new(RwLock::new(SlotState::Set(item))))
    }

    pub(crate) fn is_initialized(&self) -> bool {
        match *self.0.read().expect("Broken RwLock") {
            SlotState::Empty => false,
            SlotState::Set(_) => true,
        }
    }

    pub(crate) fn read(&self) -> anyhow::Result<T> {
        let read = self
            .0
            .read()
            .map_err(|_| anyhow!("Failed to read from RwLock"))?;
        let item = match *read {
            SlotState::Empty => return Err(anyhow!("Block not initialized when it should be")),
            SlotState::Set(ref item) => item.clone(),
        };
        Ok(item)
    }
}

pub trait SlotDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<T: Clone + SlotDebug> Debug for Slot<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slot_state = self.0.read().expect("Failed to read from RwLock");

        f.debug_struct("Atomic Block Slot")
            .field("State", &slot_state)
            .finish()
    }
}

impl<T: Clone + SlotDebug> Debug for SlotState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotState::Empty => f.write_str("Empty"),
            SlotState::Set(item) => item.fmt(f),
        }
    }
}
