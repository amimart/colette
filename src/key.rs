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

    type EncodedRef<'a>: AsRef<[u8]> + 'a
    where
        Self: 'a;

    /// Returns the encoded key.
    ///
    /// Implementations should rely if possible on the underlying bytes of the key (e.g. for
    /// fixed-size keys) to avoid unnecessary allocations.
    ///
    /// Variable-size keys should use Colette encoding helper (i.e. `encode_unsized_key_bytes`) to
    /// ensure their encoded representation remains safe for composite keys and prefix scans.
    fn encode(&self) -> Self::EncodedRef<'_>;

    /// Decodes a key from its encoded representation.
    ///
    /// Implementations should return an error if the input is malformed or
    /// incomplete.
    ///
    /// Variable-size keys should use Colette decoding helper (i.e. `decode_unsized_key_bytes`) to
    /// decode escaped and framed key bytes correctly.
    fn decode(bytes: &[u8]) -> Self::OwnedKey;
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
pub fn decode_unsized_key_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x00 => {
                let next = bytes
                    .get(i + 1)
                    .unwrap();
                match next {
                    0x00 => {
                        // terminator
                        return out;
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

impl<K: Key> Key for &K
{
    const SIZE: KeySize = K::SIZE;

    type OwnedKey = K::OwnedKey;

    type EncodedRef<'a> = K::EncodedRef<'a>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedRef<'_> {
        (*self).encode()
    }

    fn decode(bytes: &[u8]) -> Self::OwnedKey {
        K::decode(bytes)
    }
}

impl<K: Key> Key for (K,) {
    const SIZE: KeySize = K::SIZE;

    type OwnedKey = (K::OwnedKey,);

    type EncodedRef<'a> = K::EncodedRef<'a>
    where
        Self: 'a;

    fn encode(&self) -> Self::EncodedRef<'_> {
        self.0.encode()
    }

    fn decode(bytes: &[u8]) -> Self::OwnedKey {
        (K::decode(bytes),)
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

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.push(match self {
            true => 1,
            false => 0,
        });
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError> {
        let byte = *bytes.first().ok_or(DecodeKeyError::InvalidSize {
            expected: 1,
            actual: 0,
        })?;

        match byte {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(DecodeKeyError::InvalidBytes(format!(
                "invalid boolean byte: expected 0 or 1, got {value}"
            ))),
        }
    }
}

impl Key for String {
    const SIZE: KeySize = KeySize::Variable;

    fn encode_into(&self, out: &mut Vec<u8>) {
        encode_unsized_key_bytes(self.as_bytes(), out);
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
    where
        Self: Sized
    {
        let bytes = decode_unsized_key_bytes(bytes)?;
        String::from_utf8(bytes)
            .map_err(|e| DecodeKeyError::InvalidBytes(format!(
                "invalid utf-8 string bytes: {e}"
            )))
    }
}

impl<const S: usize> Key for [u8; S] {
    const SIZE: KeySize = KeySize::Fixed(S);

    fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
    where
        Self: Sized
    {
        bytes.try_into().map_err(|_| DecodeKeyError::InvalidSize {
            expected: S,
            actual: bytes.len(),
        })
    }

    fn encode(&self) -> Vec<u8> {
        self.as_ref().to_vec()
    }
}

impl Key for Vec<u8> {
    const SIZE: KeySize = KeySize::Variable;

    fn encode_into(&self, out: &mut Vec<u8>) {
        encode_unsized_key_bytes(self.as_slice(), out);
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
    where
        Self: Sized
    {
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

    fn encode_into(&self, out: &mut Vec<u8>) {
        self.0.encode_into(out);
        self.1.encode_into(out);
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
    where
        Self: Sized
    {
        Ok((
            A::decode(bytes)?,
            B::decode(bytes)?,
        ))
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

    fn encode_into(&self, out: &mut Vec<u8>) {
        self.0.encode_into(out);
        self.1.encode_into(out);
        self.2.encode_into(out);
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
    where
        Self: Sized
    {
        Ok((
            A::decode(bytes)?,
            B::decode(bytes)?,
            C::decode(bytes)?,
        ))
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

    fn encode_into(&self, out: &mut Vec<u8>) {
        self.0.encode_into(out);
        self.1.encode_into(out);
        self.2.encode_into(out);
        self.3.encode_into(out);
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
    where
        Self: Sized
    {
        Ok((
            A::decode(bytes)?,
            B::decode(bytes)?,
            C::decode(bytes)?,
            D::decode(bytes)?,
        ))
    }
}

pub trait AppendKey<PK: Key> {
    type Key<'a>: Key
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_>;
}

impl<K: Key, PK: Key> AppendKey<PK> for (K,) {
    type Key<'a> = (K, &'a PK)
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_> {
        (self.0, pk)
    }
}

impl<A: Key, B: Key, PK: Key> AppendKey<PK> for (A, B) {
    type Key<'a> = (A, B, &'a PK)
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_> {
        (self.0, self.1, pk)
    }
}

impl<A: Key, B: Key, C: Key, PK: Key> AppendKey<PK> for (A, B, C) {
    type Key<'a> = (A, B, C, &'a PK)
    where
        PK: 'a;

    fn append(self, pk: &PK) -> Self::Key<'_> {
        (self.0, self.1, self.2, pk)
    }
}
