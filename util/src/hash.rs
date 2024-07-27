//! Hash utilities.

use core::hash::{BuildHasher, Hasher};

/// Hasher for Fowler-Noll-Vo hash.
#[derive(Debug, Clone)]
pub struct FnvHasher {
    hash: u64,
}

impl FnvHasher {
    /// Constructs new [`FnvHasher`].
    pub const fn new() -> Self {
        Self {
            hash: 0xcbf2_9ce4_8422_2325,
        }
    }
}

impl Default for FnvHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for FnvHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            // exchange below
            (self.hash, _) = self.hash.overflowing_mul(0x0100_0000_01b3);
            self.hash ^= b as u64;
        }
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

/// [`BuildHasher`] for [`FnvHasher`].
#[derive(Debug, Clone, Default)]
pub struct BuildFnvHasher;

impl BuildHasher for BuildFnvHasher {
    type Hasher = FnvHasher;

    fn build_hasher(&self) -> Self::Hasher {
        FnvHasher::new()
    }
}
