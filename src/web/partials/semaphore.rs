use maud::Render;

#[must_use]
pub struct SemaphoreClass<V, T> {
    value: V,
    threshold: T,
}

impl<V, T: Default> SemaphoreClass<V, T> {
    pub fn new(value: V) -> Self {
        Self {
            value,
            threshold: Default::default(),
        }
    }
}

impl<V, T: Copy> SemaphoreClass<V, T> {
    pub const fn threshold(mut self, threshold: T) -> Self {
        self.threshold = threshold;
        self
    }
}

impl<V: PartialOrd<T>, T> Render for SemaphoreClass<V, T> {
    fn render_to(&self, buffer: &mut String) {
        if self.value > self.threshold {
            buffer.push_str("has-text-success");
        } else if self.value < self.threshold {
            buffer.push_str("has-text-danger");
        }
    }
}
