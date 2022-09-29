use maud::Render;
use num_traits::Float;
use statrs::distribution::ContinuousCDF;

pub struct CdfSemaphore<D, K, T, RL, RH, RG> {
    distribution: D,
    x: K,
    confidence_level: T,
    render_low: Option<RL>,
    render_high: Option<RH>,
    render_grey: Option<RG>,
}

impl<D, K, T, RL, RH, RG> CdfSemaphore<D, K, T, RL, RH, RG> {
    pub const fn new(distribution: D, x: K, confidence_level: T) -> Self {
        Self {
            distribution,
            x,
            confidence_level,
            render_low: None,
            render_grey: None,
            render_high: None,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn render_low(mut self, render_low: RL) -> Self {
        self.render_low = Some(render_low);
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn render_high(mut self, render_high: RH) -> Self {
        self.render_high = Some(render_high);
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn render_grey(mut self, render_grey: RG) -> Self {
        self.render_grey = Some(render_grey);
        self
    }
}

impl<D, K, T, RL, RH, RG> Render for CdfSemaphore<D, K, T, RL, RH, RG>
where
    RL: Render,
    RH: Render,
    RG: Render,
    D: ContinuousCDF<K, T>,
    K: Float,
    T: Float,
{
    fn render_to(&self, buffer: &mut String) {
        let cdf = self.distribution.cdf(self.x);
        if cdf > self.confidence_level {
            if let Some(render_low) = &self.render_low {
                render_low.render_to(buffer);
            }
        } else if T::one() - cdf > self.confidence_level {
            if let Some(render_high) = &self.render_high {
                render_high.render_to(buffer);
            }
        } else if let Some(render_grey) = &self.render_grey {
            render_grey.render_to(buffer);
        }
    }
}
