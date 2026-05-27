#[macro_export]
macro_rules! impl_unsigned_integer_key {
    ($ty:ty) => {
        impl Key for $ty {
            const SIZE: KeySize = KeySize::Fixed(std::mem::size_of::<$ty>());

            type OwnedKey = $ty;

            type EncodedRef<'a> = [u8; std::mem::size_of::<$ty>()]
            where
                Self: 'a;

            fn encode(&self) -> Self::EncodedRef<'_> {
                self.to_be_bytes()
            }

            fn decode(bytes: &[u8]) -> Self::OwnedKey {
                <$ty>::from_be_bytes(bytes.try_into().unwrap())
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

            type EncodedRef<'a> = [u8; std::mem::size_of::<$unsigned>()]
            where
                Self: 'a;

            fn encode(&self) -> Self::EncodedRef<'_> {
                let sortable = (*self as $unsigned) ^ <$signed>::MIN as $unsigned;
                sortable.to_be_bytes()
            }

            fn decode(bytes: &[u8]) -> Self::OwnedKey {
                let sortable = <$unsigned>::from_be_bytes(bytes.try_into().unwrap());
                (sortable ^ <$signed>::MIN as $unsigned) as $signed
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

            type EncodedRef<'a> = [u8; std::mem::size_of::<$int>()]
            where
                Self: 'a;

            fn encode(&self) -> Self::EncodedRef<'_> {
                match self {
                    $($variant => $value,)+
                }.to_be_bytes()
            }

            fn decode(bytes: &[u8]) -> Self::OwnedKey {
                let value = <$int>::from_be_bytes(bytes.try_into().unwrap());
                match value {
                    $($value => $variant,)+
                    _ => panic!("invalid enum discriminant {value} for type {}", stringify!($ty)),
                }
            }
        }
    };
}