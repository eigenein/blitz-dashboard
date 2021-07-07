use clap::crate_version;
use maud::{html, Markup, Render};

use crate::web::state::{RetrieveCount, State};

pub struct Footer {
    account_count: i64,
    account_snapshot_count: i64,
    tank_snapshot_count: i64,
}

impl Footer {
    pub async fn new(state: &State) -> crate::Result<Self> {
        let account_count = state.retrieve_count(RetrieveCount::Accounts).await?;
        let account_snapshot_count = state
            .retrieve_count(RetrieveCount::AccountSnapshots)
            .await?;
        let tank_snapshot_count = state.retrieve_count(RetrieveCount::TankSnapshots).await?;
        Ok(Self {
            account_count,
            account_snapshot_count,
            tank_snapshot_count,
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
                                span.icon-text.is-flex-wrap-nowrap {
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
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-heart.has-text-danger {} }
                                    span {
                                        "Создан с помощью " a href="https://www.rust-lang.org/" { "Rust" }
                                        " и " a href="https://bulma.io/" { "Bulma" }
                                    }
                                }
                            }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-id-badge.has-text-success {} }
                                    span { "Исходный код лицензирован " a href="https://opensource.org/licenses/MIT" { "MIT" } }
                                }
                            }
                        }

                        div.column."is-2" {
                            p.title."is-6" { "Поддержка" }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-comments.has-text-info {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/discussions" { "Обсуждения" } }
                                }
                            }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fab.fa-github.has-text-danger {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/issues" { "Задачи и баги" } }
                                }
                            }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-code-branch.has-text-success {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/pulls" { "Пул-реквесты" } }
                                }
                            }
                        }

                        div.column."is-3" {
                            p.title."is-6" { "Статистика" }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-user.has-text-info {} }
                                    span { strong { (self.account_count) } " аккаунтов" }
                                }
                            }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-portrait.has-text-info {} }
                                    span { strong { (self.account_snapshot_count) } " снимков аккаунтов" }
                                }
                            }
                            p."mt-1" {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-truck-monster.has-text-info {} }
                                    span { strong { (self.tank_snapshot_count) } " снимков танков" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
