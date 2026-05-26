#[macro_export]
macro_rules! impl_unsigned_integer_key {
    ($ty:ty) => {
        impl Key for $ty {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$ty>());

            fn encode_into(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }

            fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError> {
                let bytes: [u8; std::mem::size_of::<$ty>()] =
                    bytes.try_into().map_err(|_| {
                        DecodeKeyError::InvalidSize {
                            expected: std::mem::size_of::<$ty>(),
                            actual: bytes.len(),
                        }
                    })?;

                Ok(<$ty>::from_be_bytes(bytes))
            }
        }
    };
}

#[macro_export]
macro_rules! impl_signed_integer_key {
    ($signed:ty => $unsigned:ty) => {
        impl Key for $signed {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$signed>());

            fn encode_into(&self, out: &mut Vec<u8>) {
                let sortable = (*self as $unsigned) ^ <$signed>::MIN as $unsigned;
                out.extend_from_slice(&sortable.to_be_bytes());
            }

            fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError> {
                let bytes: [u8; std::mem::size_of::<$signed>()] =
                    bytes.try_into().map_err(|_| {
                        DecodeKeyError::InvalidSize {
                            expected: std::mem::size_of::<$signed>(),
                            actual: bytes.len(),
                        }
                    })?;

                let sortable = <$unsigned>::from_be_bytes(bytes);
                Ok((sortable ^ <$signed>::MIN as $unsigned) as $signed)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_enum_key {
    ($ty:ty as $int:ty { $($variant:path => $value:expr),+ $(,)? }) => {
        impl Key for $ty {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$int>());

            fn encode_into(&self, out: &mut Vec<u8>) {
                let value: $int = match self {
                    $($variant => $value,)+
                };

                value.encode_into(out);
            }

            fn decode(bytes: &[u8]) -> Result<Self, DecodeKeyError>
            where
                Self: Sized
            {
                let value = <$int>::decode(bytes)?;
                match value {
                    $($value => Ok($variant),)+
                    _ => Err(DecodeKeyError::InvalidBytes(format!(
                        "invalid enum discriminant {value} for type {}",
                        stringify!($ty)
                    ))),
                }
            }
        }
    };
}