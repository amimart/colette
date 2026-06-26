use std::collections::Bound;

pub(crate) type ScanBound = Bound<Vec<u8>>;
pub(crate) type ScanRange = (ScanBound, ScanBound);

pub(crate) trait IntoScanBounds {
    fn start_bound(&self) -> ScanBound;

    fn end_bound(&self) -> ScanBound;
}
