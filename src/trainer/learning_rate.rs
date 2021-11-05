use futures::{stream, Stream};

pub fn learning_rates(initial: f64, decay: f64, minimal: f64) -> impl Stream<Item = f64> {
    stream::unfold(1.0, move |factor| async move {
        let rate = initial / factor;
        if rate >= minimal {
            Some((rate, factor + decay))
        } else {
            Some((minimal, factor))
        }
    })
}
