use crate::Float;

pub mod statistics;
pub mod vector;

#[must_use]
#[inline]
pub fn logistic(x: Float) -> Float {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
#[cfg(nightly)]
mod benches {
    extern crate test;

    use test::{black_box, Bencher};

    use super::*;

    #[bench]
    fn bench_logistic(bencher: &mut Bencher) {
        bencher.iter(|| black_box(logistic(black_box(1.0))));
    }
}
