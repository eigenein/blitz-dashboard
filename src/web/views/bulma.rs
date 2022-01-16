use maud::Render;

/// Elements that are able to render their CSS class.
pub trait Class {
    fn render_class_to(&self, buffer: &mut String);
}

#[must_use]
#[derive(Copy, Clone)]
pub enum IconKind {
    Solid,
}

impl Class for IconKind {
    fn render_class_to(&self, buffer: &mut String) {
        let string = match self {
            Self::Solid => "fas",
        };
        buffer.push_str(string);
    }
}

#[must_use]
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum Icon {
    ArrowDown,
    ArrowUp,
    ChartArea,
    Check,
}

impl Render for Icon {
    fn render_to(&self, buffer: &mut String) {
        self.into_span().render_to(buffer)
    }
}

impl Icon {
    pub fn into_span(self) -> IconSpan {
        IconSpan::new(self)
    }
}

impl Class for Icon {
    fn render_class_to(&self, buffer: &mut String) {
        let string = match self {
            Self::ArrowDown => "fa-arrow-down",
            Self::ArrowUp => "fa-arrow-up",
            Self::ChartArea => "fa-chart-area",
            Self::Check => "fa-check",
        };
        buffer.push_str(string);
    }
}

#[must_use]
#[derive(Copy, Clone)]
pub enum Color {
    GreyLight,
    Success,
}

impl Class for Color {
    fn render_class_to(&self, buffer: &mut String) {
        let string = match self {
            Self::GreyLight => "grey-light",
            Self::Success => "success",
        };
        buffer.push_str(string);
    }
}

#[must_use]
#[derive(Copy, Clone)]
struct TextColor(Color);

impl Class for TextColor {
    fn render_class_to(&self, buffer: &mut String) {
        buffer.push_str("has-text-");
        self.0.render_class_to(buffer);
    }
}

#[must_use]
#[derive(Copy, Clone)]
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
            color.render_class_to(buffer);
        }
        buffer.push_str("\"><i class=\"");
        self.kind.render_class_to(buffer);
        buffer.push(' ');
        self.icon.render_class_to(buffer);
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
