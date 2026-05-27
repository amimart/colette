use crate::{impl_signed_integer_key, impl_unsigned_integer_key};

/// A value that can be encoded as an ordered key for Colette stores and indexes.
///
/// The encoded representation must preserve the logical ordering of the value.
///
/// Encoded keys are used for:
///
/// - primary keys;
/// - secondary indexes;
/// - range scans;
/// - prefix scans;
/// - cursor pagination.
///
/// # Ordering
///
/// Ordered KV stores compare keys lexicographically. Numeric values should
/// therefore generally use big-endian encoding.
///
/// # Variable-size keys
///
/// Variable-size values such as `str`, `String`, `[u8]` or `Vec<u8>` must use
/// escaping/framing so that composite keys remain unambiguous.
///
/// Implementations for variable-size keys should use the helper functions
/// provided by Colette to encode and decode variable-size key bytes:
/// - `encode_unsized_key_bytes`;
/// - `decode_unsized_key_bytes`;
///
/// # Composite keys
///
/// Composite keys are represented using tuples. Their encoding is obtained by
/// concatenating the encoding of each component.
///
/// # Compatibility
///
/// Changing a `Key` implementation changes the physical storage layout and
/// should be treated as a migration.
pub trait Key: Eq {
    /// The encoded size of the key.
    ///
    /// Fixed-size keys allow Colette to preallocate buffers efficiently.
    const SIZE: KeySize;

    type OwnedKey: Key + Sized;

    type EncodedBytes<'a>: AsRef<[u8]> + 'a
    where
        Self: 'a;

    /// Returns the encoded key.
    ///
    /// Implementations should rely if possible on the underlying bytes of the key (e.g. for
    /// fixed-size keys) to avoid unnecessary allocations.
    ///
    /// Variable-size keys should use Colette encoding helper (i.e. `encode_unsized_key_bytes`) to
    /// ensure their encoded representation remains safe for composite keys and prefix scans.
    fn encode(&self) -> Self::EncodedBytes<'_>;

    /// Decodes a key from its encoded representation and returns the unread bytes.
    ///
    /// Panics if the input is malformed or incomplete.
    ///
    /// Variable-size keys should use Colette decoding helper (i.e. `decode_unsized_key_bytes`) to
    /// decode escaped and framed key bytes correctly.
    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]);

    /// Decodes a key from its encoded representation.
    ///
    /// Panics if the input is malformed, incomplete, or more than complete.
    ///
    /// Variable-size keys should use Colette decoding helper (i.e. `decode_unsized_key_bytes`) to
    /// decode escaped and framed key bytes correctly.
    fn decode(bytes: &[u8]) -> Self::OwnedKey {
        let (key, rest) = Self::decode_part(bytes);
        if !rest.is_empty() {
            panic!("bytes contains more data than expected")
        }
        key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySize {
    Fixed(usize),
    Variable,
}

/// Encodes variable-size key bytes using Colette escaping and framing rules.
///
/// The `0x00` byte is reserved internally:
///
/// - `0x00 0xff` represents an escaped `0x00` byte;
/// - `0x00 0x00` marks the end of the encoded value.
///
/// This encoding ensures that variable-size keys remain safe to concatenate
/// inside composite keys while preserving lexicographic ordering.
pub fn encode_unsized_key_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    for &b in bytes {
        match b {
            0x00 => out.extend_from_slice(&[0x00, 0xff]),
            b => out.push(b),
        }
    }
    out.extend_from_slice(&[0x00, 0x00]);
}

/// Decodes variable-size key bytes encoded with
/// `encode_unsized_key_bytes`.
///
/// Returns an error if the encoded bytes contain invalid escape sequences or
/// are missing the terminating marker.
pub fn decode_unsized_key_bytes(bytes: &[u8]) -> (Vec<u8>, &[u8]) {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x00 => {
                let next = bytes.get(i + 1).unwrap();
                match next {
                    0x00 => {
                        // terminator
                        return (out, bytes.get(i + 2..).unwrap_or(&[]));
                    }
                    0xff => {
                        // escaped 0x00
                        out.push(0x00);
                        i += 2;
                    }
                    other => {
                        panic!("invalid escape sequence in encoded key bytes: expected 0x00 0x00 or 0x00 0xff, got 0x00 {other:02x}");
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    panic!("unterminated encoded key bytes: expected terminator 0x00 0x00 not found");
}

impl<K: Key> Key for &K {
    const SIZE: KeySize = K::SIZE;

    type OwnedKey = K::OwnedKey;

    type EncodedBytes<'a>
        = K::EncodedBytes<'a>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        (*self).encode()
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        K::decode_part(bytes)
    }
}

impl<K: Key> Key for (K,) {
    const SIZE: KeySize = K::SIZE;

    type OwnedKey = (K::OwnedKey,);

    type EncodedBytes<'a>
        = K::EncodedBytes<'a>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        self.0.encode()
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        let (k, r) = K::decode_part(bytes);
        ((k,), r)
    }
}

impl_unsigned_integer_key!(u8);
impl_unsigned_integer_key!(u16);
impl_unsigned_integer_key!(u32);
impl_unsigned_integer_key!(u64);
impl_unsigned_integer_key!(u128);
impl_signed_integer_key!(i8 => u8);
impl_signed_integer_key!(i16 => u16);
impl_signed_integer_key!(i32 => u32);
impl_signed_integer_key!(i64 => u64);
impl_signed_integer_key!(i128 => u128);

impl Key for bool {
    const SIZE: KeySize = KeySize::Fixed(1);

    type OwnedKey = Self;

    type EncodedBytes<'a>
        = [u8; 1]
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        match self {
            true => [1],
            false => [0],
        }
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        match bytes {
            [0, r @ ..] => (false, r),
            [1, r @ ..] => (true, r),
            _ => panic!("invalid boolean bytes"),
        }
    }
}

/// As Rust Strings are guaranteed UTF-8 we don't need escaping, so we just use `0xff` (i.e.
/// forbidden in utf-8) as end byte.
impl Key for String {
    const SIZE: KeySize = KeySize::Variable;

    type OwnedKey = Self;

    type EncodedBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        let mut out = Vec::with_capacity(self.len() + 1);
        out.extend_from_slice(self.as_bytes());
        out.push(0xff);
        out
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        let end_str_pos = bytes.iter().position(|&b| b == 0xff).unwrap();
        let (strbytes, tail) = bytes.split_at(end_str_pos);

        (
            str::from_utf8(strbytes).unwrap().to_string(),
            match tail {
                [_, r @ ..] => r,
                _ => panic!("invalid encoded string bytes"),
            },
        )
    }
}

impl<const S: usize> Key for [u8; S] {
    const SIZE: KeySize = KeySize::Fixed(S);

    type OwnedKey = Self;

    type EncodedBytes<'a>
        = &'a Self
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        self
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        let (kbytes, r) = bytes.split_at(S);
        (kbytes.try_into().unwrap(), r)
    }
}

impl Key for Vec<u8> {
    const SIZE: KeySize = KeySize::Variable;

    type OwnedKey = Self;

    type EncodedBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        let mut out = Vec::with_capacity(self.len() + 2);
        encode_unsized_key_bytes(self, &mut out);
        out
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        decode_unsized_key_bytes(bytes)
    }
}

impl<A, B> Key for (A, B)
where
    A: Key,
    B: Key,
{
    const SIZE: KeySize = match (A::SIZE, B::SIZE) {
        (KeySize::Fixed(s1), KeySize::Fixed(s2)) => KeySize::Fixed(s1 + s2),
        _ => KeySize::Variable,
    };

    type OwnedKey = (A::OwnedKey, B::OwnedKey);

    type EncodedBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        match Self::SIZE {
            KeySize::Fixed(s) => {
                let mut out = Vec::with_capacity(s);
                out.extend_from_slice(self.0.encode().as_ref());
                out.extend_from_slice(self.1.encode().as_ref());
                out
            }
            KeySize::Variable => {
                let mut out = self.0.encode().as_ref().to_vec();
                out.extend_from_slice(self.1.encode().as_ref());
                out
            }
        }
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        let (a, r) = A::decode_part(bytes);
        let (b, r) = B::decode_part(r);
        ((a, b), r)
    }
}

impl<A, B, C> Key for (A, B, C)
where
    A: Key,
    B: Key,
    C: Key,
{
    const SIZE: KeySize = match (A::SIZE, B::SIZE, C::SIZE) {
        (KeySize::Fixed(s1), KeySize::Fixed(s2), KeySize::Fixed(s3)) => {
            KeySize::Fixed(s1 + s2 + s3)
        }
        _ => KeySize::Variable,
    };

    type OwnedKey = (A::OwnedKey, B::OwnedKey, C::OwnedKey);

    type EncodedBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        match Self::SIZE {
            KeySize::Fixed(s) => {
                let mut out = Vec::with_capacity(s);
                out.extend_from_slice(self.0.encode().as_ref());
                out.extend_from_slice(self.1.encode().as_ref());
                out.extend_from_slice(self.2.encode().as_ref());
                out
            }
            KeySize::Variable => {
                let mut out = self.0.encode().as_ref().to_vec();
                out.extend_from_slice(self.1.encode().as_ref());
                out.extend_from_slice(self.2.encode().as_ref());
                out
            }
        }
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        let (a, r) = A::decode_part(bytes);
        let (b, r) = B::decode_part(r);
        let (c, r) = C::decode_part(r);
        ((a, b, c), r)
    }
}

impl<A, B, C, D> Key for (A, B, C, D)
where
    A: Key,
    B: Key,
    C: Key,
    D: Key,
{
    const SIZE: KeySize = match (A::SIZE, B::SIZE, C::SIZE, D::SIZE) {
        (KeySize::Fixed(s1), KeySize::Fixed(s2), KeySize::Fixed(s3), KeySize::Fixed(s4)) => {
            KeySize::Fixed(s1 + s2 + s3 + s4)
        }
        _ => KeySize::Variable,
    };

    type OwnedKey = (A::OwnedKey, B::OwnedKey, C::OwnedKey, D::OwnedKey);

    type EncodedBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedBytes<'_> {
        match Self::SIZE {
            KeySize::Fixed(s) => {
                let mut out = Vec::with_capacity(s);
                out.extend_from_slice(self.0.encode().as_ref());
                out.extend_from_slice(self.1.encode().as_ref());
                out.extend_from_slice(self.2.encode().as_ref());
                out.extend_from_slice(self.3.encode().as_ref());
                out
            }
            KeySize::Variable => {
                let mut out = self.0.encode().as_ref().to_vec();
                out.extend_from_slice(self.1.encode().as_ref());
                out.extend_from_slice(self.2.encode().as_ref());
                out.extend_from_slice(self.3.encode().as_ref());
                out
            }
        }
    }

    fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
        let (a, r) = A::decode_part(bytes);
        let (b, r) = B::decode_part(r);
        let (c, r) = C::decode_part(r);
        let (d, r) = D::decode_part(r);
        ((a, b, c, d), r)
    }
}

pub trait AppendKey<PK: Key> {
    type Key<'a>: Key
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_>;
}

impl<K: Key, PK: Key> AppendKey<PK> for (K,) {
    type Key<'a>
        = (K, &'a PK)
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_> {
        (self.0, pk)
    }
}

impl<A: Key, B: Key, PK: Key> AppendKey<PK> for (A, B) {
    type Key<'a>
        = (A, B, &'a PK)
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_> {
        (self.0, self.1, pk)
    }
}

impl<A: Key, B: Key, C: Key, PK: Key> AppendKey<PK> for (A, B, C) {
    type Key<'a>
        = (A, B, C, &'a PK)
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_> {
        (self.0, self.1, self.2, pk)
    }
}

#[cfg(test)]
mod tests {
    use super::Key;

    #[test]
    fn decode() {
        // u32 — unsigned big-endian
        let cases: &[(&[u8], u32)] = &[
            (&[0x00, 0x00, 0x00, 0x00], 0),
            (&[0x00, 0x00, 0x00, 0x01], 1),
            (&[0xff, 0xff, 0xff, 0xff], u32::MAX),
        ];
        for &(bytes, expected) in cases {
            assert_eq!(u32::decode(bytes), expected, "u32::decode({bytes:02x?})");
        }

        // Vec<u8> — null-escaped, terminated by [0x00, 0x00]
        let cases: &[(&[u8], &[u8])] = &[
            (&[0x00, 0x00], &[]),
            (&[0x01, 0x02, 0x00, 0x00], &[0x01, 0x02]),
            (&[0x00, 0xff, 0x00, 0x00], &[0x00]),
        ];
        for &(bytes, expected) in cases {
            assert_eq!(
                Vec::<u8>::decode(bytes),
                expected,
                "Vec<u8>::decode({bytes:02x?})"
            );
        }

        // panics when extra bytes remain after a fully decoded key
        let panic_cases: &[&dyn Fn()] = &[
            &|| {
                u32::decode(&[0x00, 0x00, 0x00, 0x01, 0xff]);
            },
            &|| {
                bool::decode(&[0x00, 0x01]);
            },
            &|| {
                String::decode(&[b'a', 0xff, b'b']);
            },
        ];
        for &case in panic_cases {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(case)).is_err(),
                "expected panic for trailing bytes"
            );
        }
    }

    #[test]
    fn encode_unsized_key_bytes() {
        // (input, expected_encoded)
        let cases: &[(&[u8], &[u8])] = &[
            (&[], &[0x00, 0x00]),
            (&[0x01, 0x02, 0x03], &[0x01, 0x02, 0x03, 0x00, 0x00]),
            (&[0xff, 0xfe], &[0xff, 0xfe, 0x00, 0x00]),
            // null bytes are escaped to [0x00, 0xff]
            (&[0x00], &[0x00, 0xff, 0x00, 0x00]),
            (&[0x00, 0x00], &[0x00, 0xff, 0x00, 0xff, 0x00, 0x00]),
            (&[0x01, 0x00, 0x02], &[0x01, 0x00, 0xff, 0x02, 0x00, 0x00]),
        ];

        for (input, expected) in cases {
            let mut out = Vec::new();
            super::encode_unsized_key_bytes(input, &mut out);
            assert_eq!(out, *expected, "encode({input:02x?})");
        }
    }

    #[test]
    fn decode_unsized_key_bytes() {
        // (encoded, expected_decoded, expected_remainder)
        let cases: &[(&[u8], &[u8], &[u8])] = &[
            (&[0x00, 0x00], &[], &[]),
            (&[0x01, 0x02, 0x03, 0x00, 0x00], &[0x01, 0x02, 0x03], &[]),
            (&[0xff, 0xfe, 0x00, 0x00], &[0xff, 0xfe], &[]),
            // escaped null bytes are decoded back to 0x00
            (&[0x00, 0xff, 0x00, 0x00], &[0x00], &[]),
            (&[0x00, 0xff, 0x00, 0xff, 0x00, 0x00], &[0x00, 0x00], &[]),
            (
                &[0x01, 0x00, 0xff, 0x02, 0x00, 0x00],
                &[0x01, 0x00, 0x02],
                &[],
            ),
            // bytes after the terminator are returned as remainder
            (&[0x00, 0x00, 0x42], &[], &[0x42]),
            (&[0x01, 0x00, 0x00, 0x02, 0x03], &[0x01], &[0x02, 0x03]),
        ];

        for (encoded, expected_value, expected_remainder) in cases {
            let (value, remainder) = super::decode_unsized_key_bytes(encoded);
            assert_eq!(value, *expected_value, "decode({encoded:02x?}) value");
            assert_eq!(
                remainder, *expected_remainder,
                "decode({encoded:02x?}) remainder"
            );
        }

        let panic_cases: &[&[u8]] = &[
            &[],           // empty — no terminator
            &[0x01, 0x02], // missing terminator
            &[0x00, 0x01], // invalid escape sequence
        ];

        for &input in panic_cases {
            assert!(
                std::panic::catch_unwind(|| super::decode_unsized_key_bytes(input)).is_err(),
                "expected panic for input {input:02x?}"
            );
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let cases: &[&[u8]] = &[
            &[],
            &[0x01, 0x02, 0x03],
            &[0xff],
            &[0x00],
            &[0x00, 0x00],
            &[0x01, 0x00, 0x02],
            &[0x00, 0x01, 0x00, 0xff, 0x00],
        ];

        for &input in cases {
            let mut encoded = Vec::new();
            super::encode_unsized_key_bytes(input, &mut encoded);
            let (decoded, remainder) = super::decode_unsized_key_bytes(&encoded);
            assert_eq!(decoded, input, "round-trip({input:02x?})");
            assert!(
                remainder.is_empty(),
                "unexpected remainder after round-trip"
            );
        }
    }

    #[test]
    fn encode_integers() {
        // u8 — big-endian (1 byte)
        let cases: &[(u8, &[u8])] = &[
            (0, &[0x00]),
            (1, &[0x01]),
            (u8::MAX, &[0xff]),
        ];
        for &(value, expected) in cases {
            assert_eq!(value.encode().as_ref(), expected, "u8::encode({value})");
        }

        // i8 — XOR with 0x80 to flip sign bit, then big-endian
        let cases: &[(i8, &[u8])] = &[
            (i8::MIN, &[0x00]),
            (-1, &[0x7f]),
            (0, &[0x80]),
            (1, &[0x81]),
            (i8::MAX, &[0xff]),
        ];
        for &(value, expected) in cases {
            assert_eq!(value.encode().as_ref(), expected, "i8::encode({value})");
        }

        // u128 — big-endian (16 bytes)
        let one_u128 = { let mut b = [0u8; 16]; b[15] = 0x01; b };
        let cases: &[(u128, [u8; 16])] = &[
            (0, [0x00; 16]),
            (1, one_u128),
            (u128::MAX, [0xff; 16]),
        ];
        for &(value, expected) in cases {
            assert_eq!(value.encode().as_ref(), expected, "u128::encode({value})");
        }

        // i128 — XOR with bit 127 to flip sign bit, then big-endian
        let zero_i128 = { let mut b = [0u8; 16]; b[0] = 0x80; b };
        let neg_one_i128 = { let mut b = [0xffu8; 16]; b[0] = 0x7f; b };
        let one_i128 = { let mut b = [0u8; 16]; b[0] = 0x80; b[15] = 0x01; b };
        let cases: &[(i128, [u8; 16])] = &[
            (i128::MIN, [0x00; 16]),
            (-1, neg_one_i128),
            (0, zero_i128),
            (1, one_i128),
            (i128::MAX, [0xff; 16]),
        ];
        for &(value, expected) in cases {
            assert_eq!(value.encode().as_ref(), expected, "i128::encode({value})");
        }
    }

    #[test]
    fn decode_part_integers() {
        // u8 — exact bytes and trailing remainder
        let cases: &[(&[u8], u8, &[u8])] = &[
            (&[0x00], 0, &[]),
            (&[0x01], 1, &[]),
            (&[0xff], u8::MAX, &[]),
            (&[0x42, 0xde, 0xad], 0x42, &[0xde, 0xad]),
        ];
        for &(bytes, expected, remainder) in cases {
            let (value, rest) = u8::decode_part(bytes);
            assert_eq!(value, expected, "u8::decode_part({bytes:02x?}) value");
            assert_eq!(rest, remainder, "u8::decode_part({bytes:02x?}) remainder");
        }

        // i8 — XOR-flipped encoding
        let cases: &[(&[u8], i8, &[u8])] = &[
            (&[0x00], i8::MIN, &[]),
            (&[0x7f], -1, &[]),
            (&[0x80], 0, &[]),
            (&[0x81], 1, &[]),
            (&[0xff], i8::MAX, &[]),
            (&[0x80, 0xde, 0xad], 0, &[0xde, 0xad]),
        ];
        for &(bytes, expected, remainder) in cases {
            let (value, rest) = i8::decode_part(bytes);
            assert_eq!(value, expected, "i8::decode_part({bytes:02x?}) value");
            assert_eq!(rest, remainder, "i8::decode_part({bytes:02x?}) remainder");
        }

        // u128 — big-endian (16 bytes)
        let one_bytes = { let mut b = [0u8; 16]; b[15] = 0x01; b };
        let u128_cases: &[(&[u8], u128, &[u8])] = &[
            (&[0x00u8; 16], 0, &[]),
            (&one_bytes, 1, &[]),
            (&[0xffu8; 16], u128::MAX, &[]),
        ];
        for &(bytes, expected, remainder) in u128_cases {
            let (value, rest) = u128::decode_part(bytes);
            assert_eq!(value, expected, "u128::decode_part value");
            assert_eq!(rest, remainder, "u128::decode_part remainder");
        }
        // with trailing bytes
        let bytes_with_tail: Vec<u8> = [0u8; 16].iter().chain(&[0xca, 0xfe]).copied().collect();
        let (value, rest) = u128::decode_part(&bytes_with_tail);
        assert_eq!(value, 0u128);
        assert_eq!(rest, &[0xca, 0xfe]);

        // i128 — XOR-flipped big-endian (16 bytes)
        let zero_enc = { let mut b = [0u8; 16]; b[0] = 0x80; b };
        let neg_one_enc = { let mut b = [0xffu8; 16]; b[0] = 0x7f; b };
        let i128_cases: &[(&[u8], i128, &[u8])] = &[
            (&[0x00u8; 16], i128::MIN, &[]),
            (&neg_one_enc, -1, &[]),
            (&zero_enc, 0, &[]),
            (&[0xffu8; 16], i128::MAX, &[]),
        ];
        for &(bytes, expected, remainder) in i128_cases {
            let (value, rest) = i128::decode_part(bytes);
            assert_eq!(value, expected, "i128::decode_part value");
            assert_eq!(rest, remainder, "i128::decode_part remainder");
        }
        // with trailing bytes
        let bytes_with_tail: Vec<u8> = [0x00u8; 16].iter().chain(&[0xbe, 0xef]).copied().collect();
        let (value, rest) = i128::decode_part(&bytes_with_tail);
        assert_eq!(value, i128::MIN);
        assert_eq!(rest, &[0xbe, 0xef]);
    }
}
