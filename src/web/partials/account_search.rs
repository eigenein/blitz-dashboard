use maud::{html, Markup};
use poem::i18n::Locale;

use crate::prelude::*;
use crate::wargaming;
use crate::web::views::search::models::{MAX_QUERY_LENGTH, MIN_QUERY_LENGTH};

pub struct AccountSearch<'a> {
    class: Option<&'a str>,
    realm: wargaming::Realm,
    value: Option<&'a str>,
    has_autofocus: bool,
    has_user_secret: bool,
    locale: &'a Locale,
}

impl<'a> AccountSearch<'a> {
    pub fn new(realm: wargaming::Realm, locale: &'a Locale) -> Self {
        Self {
            realm,
            locale,
            class: None,
            value: None,
            has_autofocus: false,
            has_user_secret: false,
        }
    }

    pub const fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    pub const fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    pub const fn has_autofocus(mut self, has_autofocus: bool) -> Self {
        self.has_autofocus = has_autofocus;
        self
    }

    pub const fn has_user_secret(mut self, has_user_secret: bool) -> Self {
        self.has_user_secret = has_user_secret;
        self
    }

    pub fn try_into_markup(self) -> Result<Markup> {
        let class = self.class.unwrap_or("");
        let markup = html! {
            div.field.has-addons {
                div.control {
                    span.select.(class) {
                        select name="realm" {
                            option
                                title=(self.locale.text("option-title-russia")?)
                                value=(wargaming::Realm::Russia.to_str())
                                selected[self.realm == wargaming::Realm::Russia]
                                { "🇷🇺" }
                            option
                                title=(self.locale.text("option-title-europe")?)
                                value=(wargaming::Realm::Europe.to_str())
                                selected[self.realm == wargaming::Realm::Europe]
                                { "🇪🇺" }
                        }
                    }
                }
                div.control.has-icons-left.is-expanded.has-icons-right[self.has_user_secret] {
                    input.input.(class)
                        type="search"
                        name="query"
                        value=(self.value.unwrap_or(""))
                        placeholder=(self.locale.text("placeholder-nickname")?)
                        autocomplete="nickname"
                        pattern="\\w+"
                        autocapitalize="none"
                        minlength=(MIN_QUERY_LENGTH)
                        maxlength=(MAX_QUERY_LENGTH)
                        spellcheck="false"
                        autocorrect="off"
                        aria-label="search"
                        aria-haspopup="false"
                        size="20"
                        autofocus[self.has_autofocus]
                        required;
                    span.icon.is-left.(class) { i class="fas fa-user" {} }
                    @if self.has_user_secret {
                        span.icon.is-right.(class) { i class="fas fa-user-secret" {} }
                    }
                }
                div.control {
                    button.button.is-link.(class) type="submit" {
                        span.icon.is-hidden-desktop { i.fas.fa-search {} }
                        span.is-hidden-touch { (self.locale.text("button-search")?) }
                    };
                }
            }
        };
        Ok(markup)
    }
}

impl TryInto<Markup> for AccountSearch<'_> {
    type Error = Error;

    fn try_into(self) -> Result<Markup> {
        self.try_into_markup()
    }
}
