use std::str::FromStr;

use poem::i18n::unic_langid::LanguageIdentifier;
use poem::i18n::I18NResources;

use crate::prelude::*;

pub fn build_resources() -> Result<I18NResources> {
    I18NResources::builder()
        .add_ftl("ru", include_str!("i18n/ru.ftl"))
        .add_ftl("en", include_str!("i18n/en.ftl"))
        .default_language(LanguageIdentifier::from_str("en")?)
        .build()
        .context("failed to build the i18n resources")
}
