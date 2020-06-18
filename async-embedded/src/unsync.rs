//! Tasks synchronization primitives that are *not* thread / interrupt safe (`!Sync`)

mod channel;
mod mutex;
mod slab;
mod waker_set;

pub use channel::Channel;
pub use mutex::Mutex;
