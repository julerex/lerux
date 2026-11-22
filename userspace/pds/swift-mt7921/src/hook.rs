//! Test hook `Mac` calls from its register accessors.

use std::cell::RefCell;

use lerux_swift::mt7921_card::Card;

thread_local! {
    static SLOT: RefCell<Option<Card>> = const { RefCell::new(None) };
}

pub(crate) struct Session;

impl Drop for Session {
    fn drop(&mut self) {
        SLOT.with(|slot| *slot.borrow_mut() = None);
    }
}

pub(crate) fn install(card: Card) -> Session {
    SLOT.with(|slot| *slot.borrow_mut() = Some(card));
    Session
}

pub(crate) fn fw_bytes() -> usize {
    SLOT.with(|slot| slot.borrow().as_ref().map_or(0, Card::fw_bytes))
}

pub(crate) fn before_read(off: u32) {
    SLOT.with(|slot| {
        if let Some(card) = slot.borrow_mut().as_mut() {
            card.before_read(off);
        }
    });
}

pub(crate) fn after_write(off: u32, value: u32) {
    SLOT.with(|slot| {
        if let Some(card) = slot.borrow_mut().as_mut() {
            card.after_write(off, value);
        }
    });
}
