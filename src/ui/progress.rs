use std::io::{self, IsTerminal, Write};

pub struct ProgressBar {
    label: String,
    total: usize,
    current: usize,
    width: usize,
    enabled: bool,
    unicode: bool,
}

impl ProgressBar {
    pub fn new(label: impl Into<String>, total: usize) -> Self {
        let mut bar = Self {
            label: label.into(),
            total: total.max(1),
            current: 0,
            width: 24,
            enabled: io::stdout().is_terminal(),
            unicode: supports_unicode_bar(),
        };
        bar.render("");
        bar
    }

    pub fn advance(&mut self, detail: impl AsRef<str>) {
        self.current = (self.current + 1).min(self.total);
        self.render(detail.as_ref());
    }

    pub fn finish(&mut self, detail: impl AsRef<str>) {
        self.current = self.total;
        self.render(detail.as_ref());
        if self.enabled {
            println!();
        }
    }

    fn render(&mut self, detail: &str) {
        if !self.enabled {
            return;
        }

        let filled = ((self.current as f32 / self.total as f32) * self.width as f32).round();
        let filled = filled.clamp(0.0, self.width as f32) as usize;
        let percent = ((self.current as f32 / self.total as f32) * 100.0).round() as usize;
        let bar = if self.unicode {
            format!(
                "{}{}",
                "█".repeat(filled),
                "░".repeat(self.width.saturating_sub(filled))
            )
        } else if filled == 0 {
            format!(">{}", " ".repeat(self.width.saturating_sub(1)))
        } else if filled >= self.width {
            "=".repeat(self.width)
        } else {
            format!(
                "{}>{}",
                "=".repeat(filled.saturating_sub(1)),
                " ".repeat(self.width.saturating_sub(filled))
            )
        };

        print!("\r{} [{}] {:>3}% {}    ", self.label, bar, percent, detail);
        let _ = io::stdout().flush();
    }
}

fn supports_unicode_bar() -> bool {
    if std::env::var("MNTPACK_ASCII_PROGRESS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return false;
    }

    if !cfg!(windows) {
        return true;
    }

    std::env::var("WT_SESSION").is_ok()
        || std::env::var("TERM_PROGRAM")
            .ok()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        || std::env::var("ConEmuANSI")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("on"))
            .unwrap_or(false)
}
