use std::collections::Bound;
use crate::prefix::Prefix;

pub(crate) type ScanBound = Bound<Vec<u8>>;
pub(crate) type ScanRange = (ScanBound, ScanBound);

pub(crate) trait IntoScanBounds {
    fn start_bound(&self) -> ScanBound;

    fn end_bound(&self) -> ScanBound;
}

impl<P: Prefix> IntoScanBounds for Bound<P> {
    fn start_bound(&self) -> Bound<Vec<u8>> {
        match self {
            Bound::Included(prefix) => prefix.start_bound(),
            Bound::Excluded(prefix) => prefix.end_bound(),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn end_bound(&self) -> Bound<Vec<u8>> {
        match self {
            Bound::Included(prefix) => prefix.end_bound(),
            Bound::Excluded(prefix) => match prefix.start_bound() {
                Bound::Included(bytes) | Bound::Excluded(bytes) => Bound::Excluded(bytes),
                Bound::Unbounded => Bound::Unbounded,
            },
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}
