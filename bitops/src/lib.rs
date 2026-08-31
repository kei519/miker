//! Provides bit-operations for integer types.

#![cfg_attr(not(test), no_std)]

use core::{
    cmp,
    ops::{Bound, RangeBounds},
};

macro_rules! impl_bit_ops {
    ($mod:ident, $t:ty) => {
        #[doc = concat!("Provides bit operations for ", stringify!($t), ".")]
        pub mod $mod {
            const fn bit_mask(bit: u32) -> $t {
                if bit == <$t>::BITS {
                    !0
                } else {
                    (1 << bit) - 1
                }
            }

            /// Returns whether `this`'s `bit`-th bit is set.
            #[inline]
            pub const fn get_bit(this: $t, bit: u32) -> bool {
                if bit < <$t>::BITS {
                    (this >> bit) & 1 == 1
                } else {
                    false
                }
            }

            /// Retunrs the `start..end` bits of `this`.
            #[inline]
            pub const fn get_bits(this: $t, start: u32, end: u32) -> $t {
                let start = if start < <$t>::BITS {
                    start
                } else {
                    <$t>::BITS
                };
                let end = if end < <$t>::BITS { end } else { <$t>::BITS };

                if end <= start {
                    return 0;
                }

                let mask = bit_mask(end) ^ bit_mask(start);
                (this & mask) >> start
            }

            /// If `value` is `true`, sets `this`'s `bit`-th bit. Otherwise, unsets.
            #[inline]
            pub const fn set_bit(this: $t, bit: u32, value: bool) -> $t {
                if bit < <$t>::BITS {
                    this & !(1 << bit) | ((value as $t) << bit)
                } else {
                    this
                }
            }

            /// Change the `start..end` bits of `this` to `value`.
            #[inline]
            pub const fn set_bits(this: $t, start: u32, end: u32, value: $t) -> $t {
                let start = if start < <$t>::BITS {
                    start
                } else {
                    <$t>::BITS
                };
                let end = if end < <$t>::BITS { end } else { <$t>::BITS };

                if end <= start {
                    return this;
                }

                let mask = bit_mask(end) ^ bit_mask(start);
                this & !mask | (value << start & mask)
            }
        }

        impl $crate::BitOps for $t {
            fn get_bit(&self, bit: u32) -> bool {
                $mod::get_bit(*self, bit)
            }

            fn get_bits(&self, bits: impl core::ops::RangeBounds<u32>) -> Self {
                let (start, end) = $crate::parse_bounds(bits, Self::BITS);
                $mod::get_bits(*self, start, end)
            }

            fn set_bit(&mut self, bit: u32, value: bool) -> &mut Self {
                *self = $mod::set_bit(*self, bit, value);
                self
            }

            fn set_bits(
                &mut self,
                bits: impl core::ops::RangeBounds<u32>,
                value: Self,
            ) -> &mut Self {
                let (start, end) = $crate::parse_bounds(bits, Self::BITS);
                *self = $mod::set_bits(*self, start, end, value);
                self
            }
        }
    };
}

/// The prelude.
pub mod prelude {
    pub use crate::{BitOps, bits_u8, bits_u16, bits_u32, bits_u64, bits_u128, bits_usize};
}

impl_bit_ops!(bits_u8, u8);
impl_bit_ops!(bits_u16, u16);
impl_bit_ops!(bits_u32, u32);
impl_bit_ops!(bits_u64, u64);
impl_bit_ops!(bits_u128, u128);
impl_bit_ops!(bits_usize, usize);

/// Provides bit operations.
pub trait BitOps {
    /// Returns whether `self`'s `bit`-th bit is set.
    fn get_bit(&self, bit: u32) -> bool;
    /// Retunrs `self`'s `bits` range bit value.
    fn get_bits(&self, bits: impl RangeBounds<u32>) -> Self;
    /// If `value` is `true`, sets `self`'s `bit`-th bit. Otherwise, unsets.
    fn set_bit(&mut self, bit: u32, value: bool) -> &mut Self;
    /// Change `self`'s `bits` range bit to `value`.
    fn set_bits(&mut self, bits: impl RangeBounds<u32>, value: Self) -> &mut Self;
}

fn parse_bounds(bounds: impl RangeBounds<u32>, width: u32) -> (u32, u32) {
    let start = match bounds.start_bound() {
        Bound::Included(&start) => start,
        Bound::Excluded(&start) => start + 1,
        Bound::Unbounded => 0,
    };
    let start = cmp::min(start, width);

    let end = match bounds.end_bound() {
        Bound::Included(&end) => end + 1,
        Bound::Excluded(&end) => end,
        Bound::Unbounded => width,
    };
    let end = cmp::min(end, width);

    (start, end)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_get_bit_fn() {
        use super::bits_u32;

        let foo = 0b1011_0010_1110_1000u32;

        let set_bits = [3, 5, 6, 7, 9, 12, 13, 15];
        for bit in set_bits {
            assert!(bits_u32::get_bit(foo, bit));
        }
        for bit in (0..u32::BITS).filter(|bit| !set_bits.contains(bit)) {
            assert!(!bits_u32::get_bit(foo, bit));
        }

        for bit in u32::BITS..u128::BITS {
            assert!(!bits_u32::get_bit(foo, bit));
        }
    }

    #[test]
    fn test_trait_get_bit() {
        use super::BitOps as _;

        let foo = 0b1011_0010_1110_1000u32;

        let set_bits = [3, 5, 6, 7, 9, 12, 13, 15];
        for bit in set_bits {
            assert!(foo.get_bit(bit));
        }
        for bit in (0..u32::BITS).filter(|bit| !set_bits.contains(bit)) {
            assert!(!foo.get_bit(bit));
        }

        for bit in u32::BITS..u128::BITS {
            assert!(!foo.get_bit(bit));
        }
    }

    #[test]
    fn test_set_bit_fn() {
        use super::bits_u32;

        let foo = 0b1011_0010_1110_1000u32;

        assert_eq!(bits_u32::set_bit(foo, 0, true), 0b1011_0010_1110_1001);
        assert_eq!(bits_u32::set_bit(foo, 0, false), 0b1011_0010_1110_1000);
        assert_eq!(bits_u32::set_bit(foo, 5, true), 0b1011_0010_1110_1000);
        assert_eq!(bits_u32::set_bit(foo, 5, false), 0b1011_0010_1100_1000);
        assert_eq!(bits_u32::set_bit(foo, 13, true), 0b1011_0010_1110_1000);
        assert_eq!(bits_u32::set_bit(foo, 13, false), 0b1001_0010_1110_1000);
        assert_eq!(bits_u32::set_bit(foo, 28, true), foo | (1 << 28));
        assert_eq!(bits_u32::set_bit(foo, 28, false), foo);

        assert_eq!(bits_u32::set_bit(foo, u32::BITS, true), foo);
        assert_eq!(bits_u32::set_bit(foo, u32::BITS, false), foo);
    }

    #[test]
    fn test_trait_set_bit() {
        use super::BitOps as _;

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bit(0, true);
        assert_eq!(foo, 0b1011_0010_1110_1001);
        foo.set_bit(0, false);
        assert_eq!(foo, 0b1011_0010_1110_1000);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bit(5, true);
        assert_eq!(foo, 0b1011_0010_1110_1000);
        foo.set_bit(5, false);
        assert_eq!(foo, 0b1011_0010_1100_1000);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bit(13, true);
        assert_eq!(foo, 0b1011_0010_1110_1000);
        foo.set_bit(13, false);
        assert_eq!(foo, 0b1001_0010_1110_1000);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bit(28, true);
        assert_eq!(foo, foo | (1 << 28));
        foo.set_bit(28, false);
        assert_eq!(foo, foo);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bit(u32::BITS, true);
        assert_eq!(foo, foo);
        foo.set_bit(u32::BITS, false);
        assert_eq!(foo, foo);
    }

    #[test]
    fn test_get_bits_fn() {
        use super::bits_u32;

        let foo = 0b1011_0010_1110_1000u32;

        assert_eq!(bits_u32::get_bits(foo, 0, 4), 0b1000);
        assert_eq!(bits_u32::get_bits(foo, 1, 7), 0b11_0100);
        assert_eq!(bits_u32::get_bits(foo, 4, 9), 0b0_1110);
        assert_eq!(bits_u32::get_bits(foo, 8, 16), 0b1011_0010);
        assert_eq!(bits_u32::get_bits(foo, 8, u32::BITS), 0b1011_0010);

        assert_eq!(bits_u32::get_bits(foo, 12, 2), 0);
    }

    #[test]
    fn test_trait_get_bits() {
        use super::BitOps as _;

        let foo = 0b1011_0010_1110_1000u32;

        assert_eq!(foo.get_bits(0..4), 0b1000);
        assert_eq!(foo.get_bits(0..=3), foo.get_bits(0..4));
        assert_eq!(foo.get_bits(..4), foo.get_bits(0..4));
        assert_eq!(foo.get_bits(..=3), foo.get_bits(0..4));

        assert_eq!(foo.get_bits(1..7), 0b11_0100);
        assert_eq!(foo.get_bits(1..=6), foo.get_bits(1..7));

        assert_eq!(foo.get_bits(4..9), 0b0_1110);
        assert_eq!(foo.get_bits(4..=8), foo.get_bits(4..9));

        assert_eq!(foo.get_bits(8..16), 0b1011_0010);
        assert_eq!(foo.get_bits(8..=15), foo.get_bits(8..16));

        assert_eq!(foo.get_bits(8..u32::BITS), 0b1011_0010);
        assert_eq!(foo.get_bits(8..), foo.get_bits(8..u32::BITS));

        assert_eq!(foo.get_bits(..), foo);

        assert_eq!(foo.get_bits(12..2), 0);
    }

    #[test]
    fn test_set_bits_fn() {
        use super::bits_u32;

        let foo = 0b1011_0010_1110_1000u32;

        assert_eq!(bits_u32::set_bits(foo, 0, 4, 0b0101), 0b1011_0010_1110_0101);
        assert_eq!(
            bits_u32::set_bits(foo, 1, 7, 0b0_0001),
            0b1011_0010_1000_0010,
        );
        assert_eq!(
            bits_u32::set_bits(foo, 4, 9, 0b1_1100),
            0b1011_0011_1100_1000,
        );
        assert_eq!(
            bits_u32::set_bits(foo, 8, 16, 0b1010_0011),
            0b1010_0011_1110_1000,
        );
        assert_eq!(
            bits_u32::set_bits(foo, 8, u32::BITS, 0b0100_1111_0000_1011),
            0b0100_1111_0000_1011_1110_1000,
        );

        assert_eq!(bits_u32::set_bits(foo, 12, 2, u32::MAX), foo);
    }

    #[test]
    fn test_trait_set_bits() {
        use super::BitOps as _;

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(0..4, 0b0101);
        assert_eq!(foo, 0b1011_0010_1110_0101);
        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(..4, 0b0101);
        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(0..=3, 0b0101);
        assert_eq!(foo, 0b1011_0010_1110_0101);
        assert_eq!(foo, 0b1011_0010_1110_0101);
        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(..=3, 0b0101);
        assert_eq!(foo, 0b1011_0010_1110_0101);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(1..7, 0b0_0001);
        assert_eq!(foo, 0b1011_0010_1000_0010);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(4..9, 0b1_1100);
        assert_eq!(foo, 0b1011_0011_1100_1000);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(8..16, 0b1010_0011);
        assert_eq!(foo, 0b1010_0011_1110_1000);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(8..u32::BITS, 0b0100_1111_0000_1011);
        assert_eq!(foo, 0b0100_1111_0000_1011_1110_1000);
        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(8.., 0b0100_1111_0000_1011);
        assert_eq!(foo, 0b0100_1111_0000_1011_1110_1000);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(.., 0xdead_beef);
        assert_eq!(foo, 0xdead_beef);

        let mut foo = 0b1011_0010_1110_1000u32;
        foo.set_bits(12..2, u32::MAX);
        assert_eq!(foo, foo);
    }
}
