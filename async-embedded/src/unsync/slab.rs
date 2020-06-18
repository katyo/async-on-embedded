// NOTE this code has been based on slab (crates.io) v0.4.2

use core::{mem, slice};
use generic_array::{ArrayLength, GenericArray};

pub mod i {
    pub struct Slab<A> {
        pub(crate) entries: Vec<A>,
        pub(crate) len: usize,
        pub(crate) next: usize,
    }

    pub struct Vec<A> {
        pub(crate) buffer: super::mem::MaybeUninit<A>,
        pub(crate) len: usize,
    }
}

/// Implementation detail
#[doc(hidden)]
pub enum Entry<T> {
    Vacant(usize),
    Occupied(T),
}

impl<A> i::Slab<A> {
    /// `Vec` `const` constructor; wrap the returned value in [`Vec`](../struct.Vec.html)
    pub const fn new() -> Self {
        Self {
            entries: i::Vec::new(),
            len: 0,
            next: 0,
        }
    }
}

impl<A> i::Vec<A> {
    /// `Vec` `const` constructor; wrap the returned value in [`Vec`](../struct.Vec.html)
    pub const fn new() -> Self {
        Self {
            buffer: mem::MaybeUninit::uninit(),
            len: 0,
        }
    }
}

impl<T, N> i::Vec<GenericArray<T, N>>
where
    N: ArrayLength<T>,
{
    pub(crate) fn as_mut_slice(&mut self) -> &mut [T] {
        // NOTE(unsafe) avoid bound checks in the slicing operation
        // &mut buffer[..len]
        unsafe { slice::from_raw_parts_mut(self.buffer.as_mut_ptr() as *mut T, self.len) }
    }

    pub(crate) fn capacity(&self) -> usize {
        N::to_usize()
    }

    pub(crate) fn push(&mut self, item: T) -> Result<(), T> {
        if self.len < self.capacity() {
            unsafe { self.push_unchecked(item) }
            Ok(())
        } else {
            Err(item)
        }
    }

    pub(crate) unsafe fn push_unchecked(&mut self, item: T) {
        // NOTE(ptr::write) the memory slot that we are about to write to is uninitialized. We
        // use `ptr::write` to avoid running `T`'s destructor on the uninitialized memory
        (self.buffer.as_mut_ptr() as *mut T)
            .add(self.len)
            .write(item);

        self.len += 1;
    }
}

/// TODO
pub struct Slab<T, N>(#[doc(hidden)] pub i::Slab<GenericArray<Entry<T>, N>>)
where
    N: ArrayLength<Entry<T>>;

impl<T, N> Slab<T, N>
where
    N: ArrayLength<Entry<T>>,
{
    pub fn new() -> Self {
        Slab(i::Slab::new())
    }

    pub fn insert(&mut self, val: T) -> Result<usize, T> {
        let key = self.0.next;
        self.insert_at(key, val)?;
        Ok(key)
    }

    fn insert_at(&mut self, key: usize, val: T) -> Result<(), T> {
        self.0.len += 1;

        if key == self.0.entries.len {
            self.0.entries.push(Entry::Occupied(val)).map_err(|entry| {
                if let Entry::Occupied(val) = entry {
                    val
                } else {
                    unreachable!()
                }
            })?;
            self.0.next = key + 1;
        } else {
            let prev = mem::replace(
                &mut self.0.entries.as_mut_slice()[key],
                Entry::Occupied(val),
            );

            match prev {
                Entry::Vacant(next) => {
                    self.0.next = next;
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, key: usize) -> T {
        // Swap the entry at the provided value
        let prev = mem::replace(
            &mut self.0.entries.as_mut_slice()[key],
            Entry::Vacant(self.0.next),
        );

        match prev {
            Entry::Occupied(val) => {
                self.0.len -= 1;
                self.0.next = key;
                val
            }
            _ => {
                // Woops, the entry is actually vacant, restore the state
                self.0.entries.as_mut_slice()[key] = prev;
                panic!("invalid key");
            }
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            entries: self.0.entries.as_mut_slice().iter_mut(),
            curr: 0,
        }
    }
}

impl<T, N> Default for Slab<T, N>
where
    N: ArrayLength<Entry<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct IterMut<'a, T> {
    entries: slice::IterMut<'a, Entry<T>>,
    curr: usize,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<(usize, &'a mut T)> {
        while let Some(entry) = self.entries.next() {
            let curr = self.curr;
            self.curr += 1;

            if let Entry::Occupied(ref mut v) = *entry {
                return Some((curr, v));
            }
        }

        None
    }
}
