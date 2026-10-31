//! Time-ordered unique ids (sortable like UUID v7, without the dependency).

use std::hash::{BuildHasher, RandomState};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_id() -> String {
    let ms = crate::ipc::now_ms();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let random = RandomState::new().hash_one((ms, n));
    format!("{ms:012x}{:012x}", random & 0xFFFF_FFFF_FFFF)
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_are_unique_and_sortable_by_time() {
        let a = super::new_id();
        let b = super::new_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 24);
        assert!(a[..12] <= b[..12]);
    }
}
