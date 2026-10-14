//! Endianness helpers that carry one declared byte order rather than each call site rolling its
//! own. The byte order is **little-endian** everywhere, so a host that reads it sees the same
//! sequence whatever its native word alignment is.

/// A type whose in-memory value can be written as a fixed-width little-endian sequence.
pub trait LeBytes: Copy {
    /// The fixed-width byte sequence.
    type Bytes: AsRef<[u8]> + Copy;

    /// Write `self` in fixed-width little-endian form.
    fn into_le_bytes(&self) -> Self::Bytes;

    /// Recover the value from a fixed-width little-endian sequence.
    fn from_le_bytes(bytes: Self::Bytes) -> Self;
}

/// Convert a value to its fixed-width little-endian sequence.
pub fn le_bytes<T: LeBytes>(value: T) -> T::Bytes {
    value.into_le_bytes()
}

/// Recover a value from a fixed-width little-endian sequence.
pub fn le_from_bytes<T: LeBytes>(value: &T::Bytes) -> T {
    T::from_le_bytes(*value)
}

impl LeBytes for u8 {
    type Bytes = [u8; 1];

    fn into_le_bytes(&self) -> Self::Bytes {
        [*self]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        bytes[0]
    }
}

impl LeBytes for i8 {
    type Bytes = [u8; 1];

    fn into_le_bytes(&self) -> Self::Bytes {
        [*self as u8]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] as i8
    }
}

impl LeBytes for u16 {
    type Bytes = [u8; 2];

    fn into_le_bytes(&self) -> Self::Bytes {
        [*self as u8, (*self >> 8) as u8]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] as u16 | ((bytes[1] as u16) << 8)
    }
}

impl LeBytes for i16 {
    type Bytes = [u8; 2];

    fn into_le_bytes(&self) -> Self::Bytes {
        [*self as u8, (*self >> 8) as u8]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        (bytes[0] as i16) | (bytes[1] as i16) << 8
    }
}

impl LeBytes for u32 {
    type Bytes = [u8; 4];

    fn into_le_bytes(&self) -> Self::Bytes {
        [*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] as u32
            | ((bytes[1] as u32) << 8)
            | ((bytes[2] as u32) << 16)
            | ((bytes[3] as u32) << 24)
    }
}

impl LeBytes for i32 {
    type Bytes = [u8; 4];

    fn into_le_bytes(&self) -> Self::Bytes {
        [*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        (bytes[0] as i32)
            | (bytes[1] as i32) << 8
            | (bytes[2] as i32) << 16
            | (bytes[3] as i32) << 24
    }
}

impl LeBytes for u64 {
    type Bytes = [u8; 8];

    fn into_le_bytes(&self) -> Self::Bytes {
        [
            *self as u8,
            (*self >> 8) as u8,
            (*self >> 16) as u8,
            (*self >> 24) as u8,
            (*self >> 32) as u8,
            (*self >> 40) as u8,
            (*self >> 48) as u8,
            (*self >> 56) as u8,
        ]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] as u64
            | ((bytes[1] as u64) << 8)
            | ((bytes[2] as u64) << 16)
            | ((bytes[3] as u64) << 24)
            | ((bytes[4] as u64) << 32)
            | ((bytes[5] as u64) << 40)
            | ((bytes[6] as u64) << 48)
            | ((bytes[7] as u64) << 56)
    }
}

impl LeBytes for i64 {
    type Bytes = [u8; 8];

    fn into_le_bytes(&self) -> Self::Bytes {
        [
            *self as u8,
            (*self >> 8) as u8,
            (*self >> 16) as u8,
            (*self >> 24) as u8,
            (*self >> 32) as u8,
            (*self >> 40) as u8,
            (*self >> 48) as u8,
            (*self >> 56) as u8,
        ]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        (bytes[0] as i64)
            | (bytes[1] as i64) << 8
            | (bytes[2] as i64) << 16
            | (bytes[3] as i64) << 24
            | (bytes[4] as i64) << 32
            | (bytes[5] as i64) << 40
            | (bytes[6] as i64) << 48
            | (bytes[7] as i64) << 56
    }
}

impl LeBytes for f32 {
    type Bytes = [u8; 4];

    fn into_le_bytes(&self) -> Self::Bytes {
        le_bytes(self.to_bits())
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        Self::from_bits(le_from_bytes::<u32>(&bytes))
    }
}

impl LeBytes for f64 {
    type Bytes = [u8; 8];

    fn into_le_bytes(&self) -> Self::Bytes {
        le_bytes(self.to_bits())
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        Self::from_bits(le_from_bytes::<u64>(&bytes))
    }
}
impl LeBytes for u128 {
    type Bytes = [u8; 16];

    fn into_le_bytes(&self) -> Self::Bytes {
        [
            *self as u8,
            (*self >> 8) as u8,
            (*self >> 16) as u8,
            (*self >> 24) as u8,
            (*self >> 32) as u8,
            (*self >> 40) as u8,
            (*self >> 48) as u8,
            (*self >> 56) as u8,
            (*self >> 64) as u8,
            (*self >> 72) as u8,
            (*self >> 80) as u8,
            (*self >> 88) as u8,
            (*self >> 96) as u8,
            (*self >> 104) as u8,
            (*self >> 112) as u8,
            (*self >> 120) as u8,
        ]
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] as u128
            | ((bytes[1] as u128) << 8)
            | ((bytes[2] as u128) << 16)
            | ((bytes[3] as u128) << 24)
            | ((bytes[4] as u128) << 32)
            | ((bytes[5] as u128) << 40)
            | ((bytes[6] as u128) << 48)
            | ((bytes[7] as u128) << 56)
            | ((bytes[8] as u128) << 64)
            | ((bytes[9] as u128) << 72)
            | ((bytes[10] as u128) << 80)
            | ((bytes[11] as u128) << 88)
            | ((bytes[12] as u128) << 96)
            | ((bytes[13] as u128) << 104)
            | ((bytes[14] as u128) << 112)
            | ((bytes[15] as u128) << 120)
    }
}

impl LeBytes for i128 {
    type Bytes = [u8; 16];

    fn into_le_bytes(&self) -> Self::Bytes {
        le_bytes(*self as u128)
    }

    fn from_le_bytes(bytes: Self::Bytes) -> Self {
        le_from_bytes::<u128>(&bytes) as i128
    }
}
