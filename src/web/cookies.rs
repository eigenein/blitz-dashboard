use chrono::{Duration, Utc};
use poem::web::cookie::{Cookie, CookieJar};
use serde::Serialize;

use crate::prelude::DateTime;

pub struct Builder(Cookie);

impl Builder {
    pub fn new(name: impl Into<String>) -> Self {
        Self(Cookie::named(name))
    }

    pub fn value(mut self, value: impl Serialize) -> Self {
        self.0.set_value(value);
        self
    }

    pub fn expires_at(mut self, expires_at: DateTime) -> Self {
        self.0.set_expires(expires_at);
        self
    }

    pub fn expires_in(self, duration: impl Into<Duration>) -> Self {
        self.expires_at(Utc::now() + duration.into())
    }

    pub fn set_path(mut self, path: impl Into<String>) -> Self {
        self.0.set_path(path);
        self
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn build(self) -> Cookie {
        self.0
    }

    pub fn add_to(self, jar: &CookieJar) {
        jar.add(self.build());
    }
}
