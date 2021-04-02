/// Taken from https://github.com/bytecodealliance/wasmtime/blob/main/crates/wiggle/borrow/src/lib.rs
/// Wasmtime is distributed under Apache License

use std::cell::RefCell;
use std::collections::HashMap;
use wiggle::{BorrowHandle, GuestError, Region};

pub struct BorrowChecker {
    bc: RefCell<InnerBorrowChecker>,
}

impl BorrowChecker {
    /// A `BorrowChecker` manages run-time validation of borrows from a `GuestMemory`. It keeps
    /// track of regions of guest memory which are possible to alias with Rust references (via the
    /// `GuestSlice` and `GuestStr` structs, which implement `std::ops::Deref` and
    /// `std::ops::DerefMut`. It also enforces that `GuestPtr::read` and `GuestPtr::write` do not
    /// access memory with an outstanding borrow.
    pub fn new() -> Self {
        BorrowChecker {
            bc: RefCell::new(InnerBorrowChecker::new()),
        }
    }
    /// Indicates whether any outstanding borrows are known to the `BorrowChecker`. This function
    /// must be `false` in order for it to be safe to recursively call into a WebAssembly module,
    /// or to manipulate the WebAssembly memory by any other means.
    pub fn has_outstanding_borrows(&self) -> bool {
        self.bc.borrow().has_outstanding_borrows()
    }

    pub(crate) fn borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.bc.borrow_mut().borrow(r)
    }
    pub(crate) fn unborrow(&self, h: BorrowHandle) {
        self.bc.borrow_mut().unborrow(h)
    }
    pub(crate) fn is_borrowed(&self, r: Region) -> bool {
        self.bc.borrow().is_borrowed(r)
    }
}

#[derive(Debug)]
/// This is a pretty naive way to account for borrows. This datastructure
/// could be made a lot more efficient with some effort.
struct InnerBorrowChecker {
    /// Map from handle to region borrowed. A HashMap is probably not ideal
    /// for this but it works. It would be more efficient if we could
    /// check `is_borrowed` without an O(n) iteration, by organizing borrows
    /// by an ordering of Region.
    borrows: HashMap<BorrowHandle, Region>,
    /// Handle to give out for the next borrow. This is the bare minimum of
    /// bookkeeping of free handles, and in a pathological case we could run
    /// out, hence [`GuestError::BorrowCheckerOutOfHandles`]
    next_handle: BorrowHandle,
}

impl InnerBorrowChecker {
    fn new() -> Self {
        InnerBorrowChecker {
            borrows: HashMap::new(),
            next_handle: BorrowHandle(0),
        }
    }

    fn has_outstanding_borrows(&self) -> bool {
        !self.borrows.is_empty()
    }

    fn is_borrowed(&self, r: Region) -> bool {
        !self.borrows.values().all(|b| !b.overlaps(r))
    }

    fn new_handle(&mut self) -> Result<BorrowHandle, GuestError> {
        // Reset handles to 0 if all handles have been returned.
        if self.borrows.is_empty() {
            self.next_handle = BorrowHandle(0);
        }
        let h = self.next_handle;
        // Get the next handle. Since we don't recycle handles until all of
        // them have been returned, there is a pathological case where a user
        // may make a Very Large (usize::MAX) number of valid borrows and
        // unborrows while always keeping at least one borrow outstanding, and
        // we will run out of borrow handles.
        self.next_handle = BorrowHandle(
            h.0.checked_add(1)
                .ok_or_else(|| GuestError::BorrowCheckerOutOfHandles)?,
        );
        Ok(h)
    }

    fn borrow(&mut self, r: Region) -> Result<BorrowHandle, GuestError> {
        if self.is_borrowed(r) {
            return Err(GuestError::PtrBorrowed(r));
        }
        let h = self.new_handle()?;
        self.borrows.insert(h, r);
        Ok(h)
    }

    fn unborrow(&mut self, h: BorrowHandle) {
        let _ = self.borrows.remove(&h);
    }
}
