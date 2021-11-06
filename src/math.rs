pub mod statistics;
pub mod vector;

#[must_use]
#[inline]
pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
#[cfg(nightly)]
mod benches {
    extern crate test;

    use super::*;
    use test::{black_box, Bencher};

    #[bench]
    fn bench_logistic(bencher: &mut Bencher) {
        bencher.iter(|| black_box(logistic(black_box(1.0))));
    }
}
