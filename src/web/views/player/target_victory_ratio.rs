use maud::{html, Markup, Render};

use crate::web::views::player::display_preferences::TargetVictoryRatio;

impl Render for TargetVictoryRatio {
    fn render(&self) -> Markup {
        html! {
            @match self {
                TargetVictoryRatio::Current => {},
                TargetVictoryRatio::P50 => { "50%" },
                TargetVictoryRatio::P55 => { "55%" },
                TargetVictoryRatio::P60 => { "60%" },
                TargetVictoryRatio::P65 => { "65%" },
                TargetVictoryRatio::P70 => { "70%" },
                TargetVictoryRatio::P75 => { "75%" },
                TargetVictoryRatio::P80 => { "80%" },
                TargetVictoryRatio::P85 => { "85%" },
                TargetVictoryRatio::P90 => { "90%" },
                TargetVictoryRatio::P95 => { "95%" },
            }
        }
    }
}
