use std::io::Write;
use std::str::FromStr;
// TODO: Functions in this module should return Result and use ? syntax.

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

pub const LIGHT_THEMES: [&str; 4] = [
    "GitHub",
    "Monokai Extended Light",
    "OneHalfLight",
    "ansi-light",
];

pub fn is_light_theme(theme: &str) -> bool {
    LIGHT_THEMES.contains(&theme)
}

const LIGHT_THEME_PLUS_COLOR: Color = Color {
    r: 0xd0,
    g: 0xff,
    b: 0xd0,
    a: 0xff,
};

const LIGHT_THEME_MINUS_COLOR: Color = Color {
    r: 0xff,
    g: 0xd0,
    b: 0xd0,
    a: 0xff,
};

const DARK_THEME_PLUS_COLOR: Color = Color {
    r: 0x01,
    g: 0x3B,
    b: 0x01,
    a: 0xff,
};

const DARK_THEME_MINUS_COLOR: Color = Color {
    r: 0x3F,
    g: 0x00,
    b: 0x01,
    a: 0xff,
};

pub struct Config<'a> {
    pub theme: &'a Theme,
    pub plus_color: Color,
    pub minus_color: Color,
    pub syntax_set: &'a SyntaxSet,
    pub width: Option<usize>,
    pub highlight_removed: bool,
    pub pager: &'a str,
}

pub fn get_config<'a>(
    syntax_set: &'a SyntaxSet,
    theme: &Option<String>,
    theme_set: &'a ThemeSet,
    user_requests_theme_for_light_terminal_background: bool,
    plus_color_str: &Option<String>,
    minus_color_str: &Option<String>,
    highlight_removed: bool,
    width: Option<usize>,
) -> Config<'a> {
    let theme_name = match theme {
        Some(ref theme) => theme,
        None => match user_requests_theme_for_light_terminal_background {
            true => "GitHub",
            false => "Monokai Extended",
        },
    };
    let minus_color = minus_color_str
        .as_ref()
        .and_then(|s| Color::from_str(s).ok());
    let plus_color = plus_color_str
        .as_ref()
        .and_then(|s| Color::from_str(s).ok());

    let is_light_theme = LIGHT_THEMES.contains(&theme_name);

    Config {
        theme: &theme_set.themes[theme_name],
        plus_color: plus_color.unwrap_or_else(|| {
            if is_light_theme {
                LIGHT_THEME_PLUS_COLOR
            } else {
                DARK_THEME_PLUS_COLOR
            }
        }),
        minus_color: minus_color.unwrap_or_else(|| {
            if is_light_theme {
                LIGHT_THEME_MINUS_COLOR
            } else {
                DARK_THEME_MINUS_COLOR
            }
        }),
        width: width,
        highlight_removed: highlight_removed,
        syntax_set: &syntax_set,
        pager: "less",
    }
}

pub struct Painter<'a> {
    pub minus_lines: Vec<String>,
    pub plus_lines: Vec<String>,
    pub writer: &'a mut Write,
    pub syntax: Option<&'a SyntaxReference>,
    pub config: &'a Config<'a>,
    pub output_buffer: String,
}

impl<'a> Painter<'a> {
    fn is_empty(&self) -> bool {
        return self.minus_lines.len() == 0 && self.plus_lines.len() == 0;
    }

    pub fn paint_and_emit_buffered_lines(&mut self) -> std::io::Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        self.paint_and_emit_text(
            self.minus_lines.join("\n"),
            Some(self.config.minus_color),
            self.config.highlight_removed,
        )?;
        self.minus_lines.clear();
        self.paint_and_emit_text(
            self.plus_lines.join("\n"),
            Some(self.config.plus_color),
            true,
        )?;
        self.plus_lines.clear();
        Ok(())
    }

    pub fn paint_and_emit_text(
        &mut self,
        text: String,
        background_color: Option<Color>,
        apply_syntax_highlighting: bool,
    ) -> std::io::Result<()> {
        paint_text(
            text,
            self.syntax.unwrap(),
            background_color,
            self.config,
            apply_syntax_highlighting,
            &mut self.output_buffer,
        );
        writeln!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.truncate(0);
        Ok(())
    }
}

// TODO: If apply_syntax_highlighting is false, then don't do
// operations related to syntax highlighting.

pub fn paint_text(
    text: String,
    syntax: &SyntaxReference,
    background_color: Option<Color>,
    config: &Config,
    apply_syntax_highlighting: bool,
    buf: &mut String,
) {
    use std::fmt::Write;
    let mut highlighter = HighlightLines::new(syntax, config.theme);

    for line in LinesWithEndings::from(&text) {
        // TODO:
        // 1. pad right
        // 2. remove +- in first column
        match background_color {
            Some(background_color) => {
                write!(
                    buf,
                    "\x1b[48;2;{};{};{}m",
                    background_color.r, background_color.g, background_color.b
                )
                .unwrap();
            }
            None => (),
        }
        let ranges: Vec<(Style, &str)> = highlighter.highlight(line, &config.syntax_set);
        paint_ranges(&ranges[..], None, apply_syntax_highlighting, buf)
    }
}

/// Based on as_24_bit_terminal_escaped from syntect
fn paint_ranges(
    foreground_style_ranges: &[(Style, &str)],
    background_color: Option<Color>,
    apply_syntax_highlighting: bool,
    buf: &mut String,
) -> () {
    for &(ref style, text) in foreground_style_ranges.iter() {
        paint(
            text,
            if apply_syntax_highlighting {
                Some(style.foreground)
            } else {
                None
            },
            background_color,
            buf,
        );
    }
}

/// Write text to buffer with color escape codes applied.
fn paint(
    text: &str,
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    buf: &mut String,
) -> () {
    use std::fmt::Write;
    match background_color {
        Some(background_color) => {
            write!(
                buf,
                "\x1b[48;2;{};{};{}m",
                background_color.r, background_color.g, background_color.b
            )
            .unwrap();
        }
        None => (),
    }
    match foreground_color {
        Some(foreground_color) => {
            write!(
                buf,
                "\x1b[38;2;{};{};{}m{}",
                foreground_color.r, foreground_color.g, foreground_color.b, text
            )
            .unwrap();
        }
        None => {
            write!(buf, "{}", text).unwrap();
        }
    }
}
