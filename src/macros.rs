#[macro_export]
macro_rules! impl_unsigned_integer_key {
    ($ty:ty) => {
        impl Key for $ty {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$ty>());

            type OwnedKey = $ty;

            type EncodedBytes<'a>
                = [u8; std::mem::size_of::<$ty>()]
            where
                Self: 'a;

            fn encode(&self) -> Self::EncodedBytes<'_> {
                self.to_be_bytes()
            }

            fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
                let (kbytes, r) = bytes.split_at(std::mem::size_of::<$ty>());
                (<$ty>::from_be_bytes(kbytes.try_into().unwrap()), r)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_signed_integer_key {
    ($signed:ty => $unsigned:ty) => {
        impl Key for $signed {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$unsigned>());

            type OwnedKey = $signed;

            type EncodedBytes<'a>
                = [u8; std::mem::size_of::<$unsigned>()]
            where
                Self: 'a;

            fn encode(&self) -> Self::EncodedBytes<'_> {
                let sortable = (*self as $unsigned) ^ <$signed>::MIN as $unsigned;
                sortable.to_be_bytes()
            }

            fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
                let (kbytes, r) = bytes.split_at(std::mem::size_of::<$unsigned>());
                let sortable = <$unsigned>::from_be_bytes(kbytes.try_into().unwrap());
                ((sortable ^ <$signed>::MIN as $unsigned) as $signed, r)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_enum_key {
    ($ty:ty as $int:ty { $($variant:path => $value:expr),+ $(,)? }) => {
        impl Key for $ty {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$int>());

            type OwnedKey = Self;

            type EncodedBytes<'a> = [u8; std::mem::size_of::<$int>()]
            where
                Self: 'a;

            fn encode(&self) -> Self::EncodedBytes<'_> {
                let v: $int = match self {
                    $($variant => $value,)+
                };
                v.to_be_bytes()
            }

            fn decode_part(bytes: &[u8]) -> (Self::OwnedKey, &[u8]) {
                let (kbytes, r) = bytes.split_at(std::mem::size_of::<$int>());
                let value = <$int>::from_be_bytes(kbytes.try_into().unwrap());
                (match value {
                    $($value => $variant,)+
                    _ => panic!("invalid enum discriminant {value} for type {}", stringify!($ty)),
                }, r)
            }
        }
    };
}
