use human_repr::HumanCount;
use maud::{html, Markup, Render};

pub struct HumanFloat<T>(pub T);

impl<T: HumanCount + Render + Copy> Render for HumanFloat<T> {
    fn render(&self) -> Markup {
        html! {
            span title=(self.0) { (self.0.human_count_bare()) }
        }
    }
}
