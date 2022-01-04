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
    color: Option<Color>,
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
}
