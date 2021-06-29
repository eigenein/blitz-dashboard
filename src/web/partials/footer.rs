use clap::crate_version;
use maud::{html, Markup, Render};

use crate::database::Statistics as DatabaseStatistics;
use crate::web::state::State;

pub struct Footer {
    database_statistics: DatabaseStatistics,
}

impl Footer {
    pub async fn new(state: &State) -> crate::Result<Self> {
        let database = state.database.clone();
        let database_statistics =
            async_std::task::spawn(async move { database.lock().await.retrieve_statistics() })
                .await?;
        Ok(Self {
            database_statistics,
        })
    }
}

impl Render for Footer {
    fn render(&self) -> Markup {
        html! {
            footer.footer {
                div.container {
                    div.columns {
                        div.column."is-3" {
                            p.title."is-6" { "О проекте" }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-home.has-text-info {} }
                                    span {
                                        a href="https://github.com/eigenein/blitz-dashboard" {
                                            "Blitz Dashboard " (crate_version!())
                                        }
                                        " © "
                                        a href="https://github.com/eigenein" { "@eigenein" }
                                    }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-heart.has-text-danger {} }
                                    span {
                                        "Создан с помощью " a href="https://www.rust-lang.org/" { "Rust" }
                                        " и " a href="https://bulma.io/" { "Bulma" }
                                    }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-id-badge.has-text-success {} }
                                    span { "Исходный код лицензирован " a href="https://opensource.org/licenses/MIT" { "MIT" } }
                                }
                            }
                        }

                        div.column."is-2" {
                            p.title."is-6" { "Поддержка" }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-comments.has-text-info {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/discussions" { "Обсуждения" } }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fab.fa-github.has-text-danger {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/issues" { "Задачи и баги" } }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-code-branch.has-text-success {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/pulls" { "Пул-реквесты" } }
                                }
                            }
                        }

                        div.column."is-3" {
                            p.title."is-6" { "Статистика" }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-user.has-text-info {} }
                                    span { strong { (self.database_statistics.account_count) } " аккаунтов" }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-portrait.has-text-info {} }
                                    span { strong { (self.database_statistics.account_snapshot_count) } " снимков аккаунтов" }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-truck-monster.has-text-info {} }
                                    span { strong { (self.database_statistics.tank_snapshot_count) } " снимков танков" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
