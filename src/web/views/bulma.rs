use maud::Render;

#[must_use]
pub enum IconKind {
    Solid,
}

impl Render for IconKind {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(match self {
            Self::Solid => "fas",
        });
    }
}

#[must_use]
#[allow(dead_code)]
pub enum Icon {
    ArrowDown,
    ArrowUp,
    ChartArea,
}

impl Render for Icon {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(match self {
            Self::ArrowDown => "fa-arrow-down",
            Self::ArrowUp => "fa-arrow-up",
            Self::ChartArea => "fa-chart-area",
        });
    }
}

impl Icon {
    pub fn into_span(self) -> IconSpan {
        IconSpan::new(self)
    }
}

#[must_use]
pub enum Color {
    GreyLight,
}

impl Render for Color {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(match self {
            Self::GreyLight => "grey-light",
        });
    }
}

#[must_use]
struct TextColor(Color);

impl Render for TextColor {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str("has-text-");
        self.0.render_to(buffer);
    }
}

#[must_use]
pub struct IconSpan {
    icon: Icon,
    kind: IconKind,
    color: Option<TextColor>,
}

impl IconSpan {
    fn new(icon: Icon) -> Self {
        Self {
            icon,
            kind: IconKind::Solid,
            color: None,
        }
    }

    #[allow(dead_code)]
    pub fn kind(mut self, kind: IconKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(TextColor(color));
        self
    }
}

impl Render for IconSpan {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str("<span class=\"icon");
        if let Some(color) = &self.color {
            buffer.push(' ');
            color.render_to(buffer);
        }
        buffer.push_str("\"><i class=\"");
        self.kind.render_to(buffer);
        buffer.push(' ');
        self.icon.render_to(buffer);
        buffer.push_str("\"></i></span>");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_span_ok() {
        let mut buffer = String::new();
        Icon::ChartArea
            .into_span()
            .color(Color::GreyLight)
            .render_to(&mut buffer);
        assert_eq!(
            &buffer,
            // language=html
            r#"<span class="icon has-text-grey-light"><i class="fas fa-chart-area"></i></span>"#,
        );
    }
}
