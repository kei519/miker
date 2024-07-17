use core::ops::RangeBounds;

pub trait BitField {
    /// Returns whether `self`'s `bit`-th bit is set.
    fn get_bit(&self, bit: u32) -> bool;
    /// Retunrs `self`'s `bits` range bit value.
    fn get_bits(&self, bits: impl RangeBounds<u32>) -> Self;
    /// If `value` is `true`, sets `self`'s `bit`-th bit. Otherwise, unsets.
    fn set_bit(&mut self, bit: u32, value: bool);
    /// Change `self`'s `bits` range bit to `value`.
    fn set_bits(&mut self, bits: impl RangeBounds<u32>, value: Self);
}

crate::impl_bit_field!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize);

/// Implements standard [BitField] implementation to the designated types that have bit operations
/// and the constant `BITS`.
#[macro_export]
macro_rules! impl_bit_field {
    () => {};
    (,) => {};
    ($t:ty) => {
        $crate::impl_bit_field!($t,);
    };
    ($t:ty, $( $u:tt ),*) => {
        impl BitField for $t {
            fn get_bit(&self, bit: u32) -> bool {
                if bit > Self::BITS {
                    return false;
                }

                if (*self >> bit) & 0x01 != 0 {
                    true
                } else {
                    false
                }
            }

            fn get_bits(&self, bits: impl core::ops::RangeBounds<u32>) -> Self {
                let start = match bits.start_bound() {
                    core::ops::Bound::Included(&start) => start,
                    core::ops::Bound::Excluded(&start) => start + 1,
                    core::ops::Bound::Unbounded => 0,
                };
                let start = core::cmp::min(start, Self::BITS);

                let end = match bits.end_bound() {
                    core::ops::Bound::Included(&end) => end + 1,
                    core::ops::Bound::Excluded(&end) => end,
                    core::ops::Bound::Unbounded => Self::BITS,
                };
                let end = core::cmp::min(end, Self::BITS);

                let mut mask = 0;
                for _ in start..end {
                    mask <<= 1;
                    mask |= 1;
                }

                *self >> start & mask
            }

            fn set_bit(&mut self, bit: u32, value: bool) {
                if bit > Self::BITS {
                    return;
                }

                let mask = 1 << bit;
                *self = *self & !mask | Self::from(value) << bit;
            }

            fn set_bits(&mut self, bits: impl core::ops::RangeBounds<u32>, value: Self) {
                let start = match bits.start_bound() {
                    core::ops::Bound::Included(&start) => start,
                    core::ops::Bound::Excluded(&start) => start + 1,
                    core::ops::Bound::Unbounded => 0,
                };
                let start = core::cmp::min(start, Self::BITS);

                let end = match bits.end_bound() {
                    core::ops::Bound::Included(&end) => end + 1,
                    core::ops::Bound::Excluded(&end) => end,
                    core::ops::Bound::Unbounded => Self::BITS,
                };
                let end = core::cmp::min(end, Self::BITS);

                let mut mask = 0;
                for bit in start..end {
                    mask |= 1 << bit;
                }

                *self = *self & !mask | value << start & mask;
            }
        }

        $crate::impl_bit_field!($( $u ),*);
    };
}
