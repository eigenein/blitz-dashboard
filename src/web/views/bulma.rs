use maud::Render;

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

struct IconSpan {
    icon: Icon,
    kind: IconKind,
    color: Option<Color>,
}

impl IconSpan {
    fn new(icon: Icon) -> Self {
        Self {
            icon,
            kind: IconKind::Solid,
            color: None,
        }
    }
}
