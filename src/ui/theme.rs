use ratatui::style::Color;

pub struct Theme {
    /// One color per command, reused in order when there are more commands.
    pub tabs: [Color; 6],
    pub accent: Color,
    pub border: Color,
    pub rule: Color,
    pub muted: Color,
    pub key: Color,
    pub hint: Color,
    pub warn: Color,
    pub error: Color,
    pub ok: Color,
    pub input: Color,
}

impl Theme {
    /// Pastel 24-bit colors where the terminal supports them, and the
    /// terminal's own 16 colors elsewhere.
    pub fn detect() -> &'static Self {
        let truecolor = std::env::var("COLORTERM")
            .is_ok_and(|v| v.eq_ignore_ascii_case("truecolor") || v.eq_ignore_ascii_case("24bit"));
        // Windows consoles handle 24-bit color but don't set `COLORTERM`.
        if truecolor || cfg!(windows) {
            &RGB
        } else {
            &ANSI
        }
    }

    pub fn tab(&self, index: usize) -> Color {
        self.tabs[index % self.tabs.len()]
    }
}

static RGB: Theme = Theme {
    tabs: [
        Color::Rgb(0x93, 0xc5, 0xfd),
        Color::Rgb(0xc4, 0xb5, 0xfd),
        Color::Rgb(0xfb, 0x71, 0x85),
        Color::Rgb(0xfd, 0xba, 0x74),
        Color::Rgb(0x86, 0xef, 0xac),
        Color::Rgb(0xfc, 0xd3, 0x4d),
    ],
    accent: Color::Rgb(0x93, 0xc5, 0xfd),
    border: Color::Rgb(0x44, 0x44, 0x4c),
    rule: Color::Rgb(0x66, 0x66, 0x66),
    muted: Color::Rgb(0x88, 0x88, 0x88),
    key: Color::Rgb(0xd4, 0xd4, 0xd8),
    hint: Color::Rgb(0xa1, 0xa1, 0xaa),
    warn: Color::Rgb(0xe5, 0xc0, 0x7b),
    error: Color::Rgb(0xef, 0x44, 0x44),
    ok: Color::Rgb(0x86, 0xef, 0xac),
    input: Color::Rgb(0xd8, 0xb4, 0xfe),
};

static ANSI: Theme = Theme {
    tabs: [
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightRed,
        Color::LightYellow,
        Color::LightGreen,
        Color::LightCyan,
    ],
    accent: Color::LightBlue,
    border: Color::DarkGray,
    rule: Color::DarkGray,
    muted: Color::DarkGray,
    key: Color::White,
    hint: Color::Gray,
    warn: Color::Yellow,
    error: Color::Red,
    ok: Color::Green,
    input: Color::Magenta,
};
