use maud::Render;

#[must_use]
pub struct SemaphoreClass<T> {
    value: T,
    threshold: T,
}

impl<T: Default> SemaphoreClass<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            threshold: Default::default(),
        }
    }
}

impl<T: Copy> SemaphoreClass<T> {
    pub const fn threshold(mut self, threshold: T) -> Self {
        self.threshold = threshold;
        self
    }
}

impl<T: PartialOrd> Render for SemaphoreClass<T> {
    fn render_to(&self, buffer: &mut String) {
        if self.value > self.threshold {
            buffer.push_str("has-text-success");
        } else if self.value < self.threshold {
            buffer.push_str("has-text-danger");
        }
    }
}
