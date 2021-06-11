use clap::crate_version;
use maud::{html, Markup, Render};

use crate::web::state::State;

pub struct Footer {
    account_count: i64,
    account_snapshot_count: i64,
    tank_snapshot_count: i64,
}

impl Footer {
    pub async fn new(state: &State) -> crate::Result<Self> {
        let account_count = state.database.get_account_count().await?;
        let account_snapshot_count = state.database.get_account_snapshot_count().await?;
        let tank_snapshot_count = state.database.get_tank_snapshot_count().await?;
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
                            p.title."is-6" { "About" }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-home.has-text-info {} }
                                    span {
                                        a href="https://github.com/eigenein/blitz-dashboard" {
                                            "Blitz Dashboard " (crate_version!())
                                        }
                                        " by "
                                        a href="https://github.com/eigenein" { "@eigenein" }
                                    }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-heart.has-text-danger {} }
                                    span {
                                        "Made with " a href="https://www.rust-lang.org/" { "Rust" }
                                        " and " a href="https://bulma.io/" { "Bulma" }
                                    }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-id-badge.has-text-success {} }
                                    span { "Source code licensed " a href="https://opensource.org/licenses/MIT" { "MIT" } }
                                }
                            }
                        }

                        div.column."is-2" {
                            p.title."is-6" { "Support" }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-comments.has-text-info {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/discussions" { "Discussions" } }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fab.fa-github.has-text-danger {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/issues" { "Issues" } }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-code-branch.has-text-success {} }
                                    span { a href="https://github.com/eigenein/blitz-dashboard/pulls" { "Pull requests" } }
                                }
                            }
                        }

                        div.column."is-3" {
                            p.title."is-6" { "Statistics" }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-user.has-text-info {} }
                                    span { strong { (self.account_count) } " accounts" }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-portrait.has-text-info {} }
                                    span { strong { (self.account_snapshot_count) } " account snapshots" }
                                }
                            }
                            p."mt-1" {
                                span.icon-text {
                                    span.icon { i.fas.fa-truck-monster.has-text-info {} }
                                    span { strong { (self.tank_snapshot_count) } " tank snapshots" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
