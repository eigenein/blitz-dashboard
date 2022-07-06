use rocket::http::uri::fmt::{FromUriParam, Part};
use rocket::request::FromParam;

use crate::wargaming;

impl<P: Part> FromUriParam<P, wargaming::Realm> for wargaming::Realm {
    type Target = &'static str;

    fn from_uri_param(param: wargaming::Realm) -> Self::Target {
        param.to_str()
    }
}

impl FromParam<'_> for wargaming::Realm {
    type Error = anyhow::Error;

    fn from_param(param: &str) -> Result<Self, Self::Error> {
        Self::try_from(param)
    }
}
