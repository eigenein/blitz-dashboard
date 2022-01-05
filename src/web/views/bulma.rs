use maud::Render;

#[allow(dead_code)]
enum IconKind {
    Solid,
}

impl Render for IconKind {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(match self {
            Self::Solid => "fas",
        });
    }
}

#[allow(dead_code)]
enum Icon {
    ChartArea,
}

impl Render for Icon {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(match self {
            Self::ChartArea => "fa-chart-area",
        });
    }
}

#[allow(dead_code)]
enum Color {
    GreyLight,
}

impl Render for Color {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str(match self {
            Self::GreyLight => "grey-light",
        });
    }
}

struct TextColor(Color);

impl Render for TextColor {
    fn render_to(&self, buffer: &mut String) {
        buffer.push_str("has-text-");
        self.0.render_to(buffer);
    }
}

#[allow(dead_code)]
struct IconSpan {
    icon: Icon,
    kind: IconKind,
    color: Option<TextColor>,
}

impl IconSpan {
    #[allow(dead_code)]
    fn new(icon: Icon) -> Self {
        Self {
            icon,
            kind: IconKind::Solid,
            color: None,
        }
    }

    #[allow(dead_code)]
    fn kind(&mut self, kind: IconKind) -> &mut Self {
        self.kind = kind;
        self
    }

    #[allow(dead_code)]
    fn color(&mut self, color: Color) -> &mut Self {
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
        IconSpan::new(Icon::ChartArea)
            .color(Color::GreyLight)
            .render_to(&mut buffer);
        assert_eq!(
            &buffer,
            // language=html
            r#"<span class="icon has-text-grey-light"><i class="fas fa-chart-area"></i></span>"#,
        );
    }
}
