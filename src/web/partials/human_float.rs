use human_repr::HumanCount;
use maud::{display, html, Markup, Render};

pub struct HumanFloat(pub f64);

impl Render for HumanFloat {
    fn render(&self) -> Markup {
        html! {
            @if self.0.is_finite() {
                span title=(self.0) { (display(self.0.human_count_bare())) }
            } @else {
                span { "-" }
            }
        }
    }
}
