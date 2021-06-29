use maud::{html, Markup};

#[inline(always)]
pub fn format_f64(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
    }
}
