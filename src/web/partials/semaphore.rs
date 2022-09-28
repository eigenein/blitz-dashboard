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

#[must_use]
pub struct SemaphoreOptionalClass<R, V, T> {
    value: V,
    threshold: T,
    render: R,
}

impl<R: Default, V, T: Default> SemaphoreOptionalClass<R, V, T> {
    pub fn new(value: V, render: R) -> Self {
        Self {
            value,
            threshold: Default::default(),
            render,
        }
    }
}

impl<R, V, T: Copy> SemaphoreOptionalClass<R, V, T> {
    pub const fn threshold(mut self, threshold: T) -> Self {
        self.threshold = threshold;
        self
    }
}

impl<R: Render, V: PartialOrd<T>, T> Render for SemaphoreOptionalClass<R, V, T> {
    fn render_to(&self, buffer: &mut String) {
        if self.value > self.threshold {
            self.render.render_to(buffer);
        }
    }
}
