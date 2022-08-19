use maud::{html, Markup, Render};

use crate::math::statistics::ConfidenceLevel;

impl Render for ConfidenceLevel {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::Z80 => { "80" },
                Self::Z85 => { "85" },
                Self::Z87 => { "87" },
                Self::Z88 => { "88" },
                Self::Z89 => { "89" },
                Self::Z90 => { "90" },
                Self::Z95 => { "95" },
                Self::Z96 => { "96" },
                Self::Z97 => { "97" },
                Self::Z98 => { "98" },
                Self::Z99 => { "99" },
            }
            span.has-text-grey { "%" }
        }
    }
}
