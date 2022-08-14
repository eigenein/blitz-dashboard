use maud::Render;

pub struct SemaphoreClass<T> {
    value: T,
    threshold: T,
}

impl<T> SemaphoreClass<T> {
    pub const fn new(value: T, threshold: T) -> Self {
        Self { value, threshold }
    }
}

impl<T: PartialOrd> Render for SemaphoreClass<T> {
    fn render_to(&self, buffer: &mut String) {
        if self.value > self.threshold {
            buffer.push_str("has-text-success");
        } else {
            buffer.push_str("has-text-danger");
        }
    }
}
