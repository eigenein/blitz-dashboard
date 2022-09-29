use maud::Render;
use num_traits::Float;
use statrs::distribution::ContinuousCDF;

pub struct CdfSemaphore<D, K, T, RL, RH> {
    pub distribution: D,
    pub x: K,
    pub confidence_level: T,
    pub render_low: RL,
    pub render_high: RH,
}

impl<D, K, T, RL, RH> Render for CdfSemaphore<D, K, T, RL, RH>
where
    RL: Render,
    RH: Render,
    D: ContinuousCDF<K, T>,
    K: Float,
    T: Float,
{
    fn render_to(&self, buffer: &mut String) {
        let cdf = self.distribution.cdf(self.x);
        if cdf > self.confidence_level {
            self.render_low.render_to(buffer);
        } else if T::one() - cdf > self.confidence_level {
            self.render_high.render_to(buffer);
        }
    }
}
