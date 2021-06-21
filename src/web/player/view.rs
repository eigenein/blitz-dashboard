use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup, Render};
use tide::StatusCode;

use crate::models::Vehicle;
use crate::web::components::footer::Footer;
use crate::web::components::icon_text;
use crate::web::partials::header;
use crate::web::player::model::PlayerViewModel;
use crate::web::responses::render_document;
use crate::web::state::State;

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = PlayerViewModel::new(&request).await?;
    let footer = Footer::new(&request.state()).await?;
    Ok(render_document(
        StatusCode::Ok,
        Some(model.nickname.as_str()),
        html! {
            (header(model.account_id))

            section.section {
                div.container {
                    div.tile.is-ancestor {
                        div.tile."is-6".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-user", &model.nickname)) }
                                }
                                div.card-content {
                                    div.level {
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Account age" }
                                                p.title title=(model.created_at) {
                                                    (HumanTime::from(model.created_at).to_text_en(Accuracy::Rough, Tense::Present))
                                                }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Tanks played" }
                                                p.title { (model.n_tanks) }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Battles" }
                                                p.title { (model.n_battles) }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Last battle" }
                                                p.title.(if model.has_recently_played { "has-text-success" } else if model.is_inactive { "has-text-danger" } else { "" })
                                                    title=(model.last_battle_time) {
                                                    (HumanTime::from(model.last_battle_time))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div.tabs.is-boxed {
                        ul {
                            li.is-active { a { "Today" } }
                            li { a { "Week" } }
                            li { a { "Month" } }
                            li { a { "Year" } }
                        }
                    }

                    div.tile.is-ancestor {
                        div.tile."is-2".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-sort-numeric-up-alt", "Battles")) }
                                }
                                div.card-content {
                                    div.level {
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Battles" }
                                                p.title { "1234" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div.tile.is-ancestor {
                        div.tile."is-4".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-percentage", "Wins")) }
                                }
                                div.card-content {
                                    div.level {
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Low" }
                                                p.title { "10.3%" }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Wins" }
                                                p.title { "12.3%" }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "High" }
                                                p.title { "14.3%" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div.tile."is-4".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-heart", "Survival")) }
                                }
                                div.card-content {
                                    div.level {
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Low" }
                                                p.title { "10.3%" }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Wins" }
                                                p.title { "12.3%" }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "High" }
                                                p.title { "14.3%" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div.tile."is-4".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-bullseye", "Hits")) }
                                }
                                div.card-content {
                                    div.level {
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Low" }
                                                p.title { "10.3%" }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "Wins" }
                                                p.title { "12.3%" }
                                            }
                                        }
                                        div.level-item.has-text-centered {
                                            div {
                                                p.heading { "High" }
                                                p.title { "14.3%" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            (footer)
        },
    ))
}

pub fn get_account_url(account_id: i32) -> String {
    format!("/ru/{}", account_id)
}

impl Render for Vehicle {
    fn render(&self) -> Markup {
        let tier = match self.tier {
            1 => "Ⅰ",
            2 => "Ⅱ",
            3 => "Ⅲ",
            4 => "Ⅳ",
            5 => "Ⅴ",
            6 => "Ⅵ",
            7 => "Ⅶ",
            8 => "Ⅷ",
            9 => "Ⅸ",
            10 => "Ⅹ",
            _ => "?",
        };
        html! {
            strong.(if self.is_premium { "has-text-warning-dark" } else { "" }) title=(self.tank_id) {
                (tier) " " (self.name)
            }
        }
    }
}
