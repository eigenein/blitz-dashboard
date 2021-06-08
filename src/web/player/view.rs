use chrono_humanize::HumanTime;
use maud::html;
use tide::StatusCode;

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
                        div.tile."is-4".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-user", "Player")) }
                                }
                                div.card-content {
                                    h1.title { (model.nickname) }
                                    h2.subtitle title=(model.created_at) {
                                        "created " (HumanTime::from(model.created_at))
                                    }
                                }
                            }
                        }

                        div class="tile is-8 is-parent" {
                            div class="tile is-child card" {
                                header class="card-header" {
                                    p class="card-header-title" { (icon_text("fas fa-table", "Overview")) }
                                }
                                div class="card-content" {
                                    div.level {
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Battles" }
                                                p.title { (model.n_battles) }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Wins" }
                                                p.title {
                                                    span class=(win_percentage_class(model.wins)) {
                                                        (format!("{:.1}", model.wins)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Survival" }
                                                p.title {
                                                    (format!("{:.1}", model.survival)) "%"
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
                                            div {
                                                p.heading { "Hits" }
                                                p.title {
                                                    span {
                                                        (format!("{:.1}", model.hits)) "%"
                                                    }
                                                }
                                            }
                                        }
                                        div class="level-item has-text-centered" {
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

                    div.tile.is-ancestor {
                        div.tile."is-4".is-parent {
                            div.tile.is-child.card {
                                header.card-header {
                                    p.card-header-title { (icon_text("fas fa-truck-monster", "Vehicles")) }
                                }
                                div.card-content {
                                    table.table.is-striped.is-hoverable.is-fullwidth {
                                        tbody {
                                            @if let Some(tank) = model.longest_life_time_tank {
                                                tr {
                                                    td { "Most lived tank" }
                                                    td { (tank.tank_id) }
                                                }
                                            }
                                            @if let Some(tank) = model.most_played_tank {
                                                tr {
                                                    td { "Most played tank" }
                                                    td { (tank.tank_id) }
                                                }
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

fn win_percentage_class(percentage: f32) -> &'static str {
    if percentage < 45.0 {
        "has-text-danger"
    } else if percentage < 50.0 {
        "has-text-warning"
    } else if percentage < 60.0 {
        "has-text-primary"
    } else {
        "has-text-success"
    }
}
