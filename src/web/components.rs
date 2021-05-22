use maud::{html, Markup};

/// HTML components.

/// Documentation: <https://bulma.io/documentation/components/card/>.
pub fn card(title: Option<Markup>, content: Option<Markup>) -> Markup {
    html! {
        div.card {
            @if let Some(title) = title {
                header class="card-header" {
                    p class="card-header-title" { (title) }
                }
            }
            @if let Some(content) = content {
                div class="card-content" {
                    p.content { (content) }
                }
            }
        }
    }
}
