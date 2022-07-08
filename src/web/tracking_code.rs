use std::fmt::Write;

use maud::{PreEscaped, Render};

use crate::opts::WebOpts;
use crate::prelude::*;

#[must_use]
#[derive(Clone)]
pub struct TrackingCode(PreEscaped<String>);

impl TrackingCode {
    pub fn new(opts: &WebOpts) -> Result<Self> {
        let mut code = String::new();
        if let Some(measurement_id) = &opts.gtag {
            write!(
                code,
                r#"<!-- Global site tag (gtag.js) - Google Analytics --> <script async src="https://www.googletagmanager.com/gtag/js?id=G-S1HXCH4JPZ"></script> <script>window.dataLayer = window.dataLayer || []; function gtag(){{dataLayer.push(arguments);}} gtag('js', new Date()); gtag('config', '{}'); </script>"#,
                measurement_id,
            )?;
        };
        Ok(Self(PreEscaped(code)))
    }
}

impl Render for &TrackingCode {
    fn render_to(&self, buffer: &mut String) {
        self.0.render_to(buffer);
    }
}
