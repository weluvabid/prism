use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use resvg::usvg;
use tiny_skia::{Pixmap, Transform};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::{ChromeStyle, RenderSettings},
    normalize::{self, MAX_LINES},
    theme::Theme,
};

const OUTER_PADDING: u32 = 48;
const CHROME_HEIGHT: u32 = 54;
const MAX_DIMENSION: u32 = 16_384;
const MAX_PIXELS: u64 = 32_000_000;
const MAX_RENDER_TEXT_BYTES: usize = 16 * 1024 * 1024;
const FONT_FAMILY: &str =
    "JetBrains Mono, SFMono-Regular, Menlo, Consolas, Liberation Mono, monospace";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderStats {
    pub width: u32,
    pub height: u32,
    pub lines: usize,
}

pub fn render_png(
    input: &str,
    theme: &Theme,
    settings: &RenderSettings,
    output: &Path,
) -> Result<RenderStats> {
    settings.validate()?;
    let normalized = normalize::normalize(input, settings.tab_width, MAX_RENDER_TEXT_BYTES)?;
    if normalized.trim().is_empty() {
        bail!("input contains no visible text after normalization");
    }
    if normalized.len() > MAX_RENDER_TEXT_BYTES {
        bail!("normalized text exceeds the 16 MiB rendering limit; reduce the supplied content");
    }

    let physical_lines = normalized.split('\n').count();
    if physical_lines > MAX_LINES {
        bail!("input has more than {MAX_LINES} lines; reduce the input before rendering");
    }

    let width = settings.width;
    let card_width = width - OUTER_PADDING * 2;
    let available_width = card_width.saturating_sub(u32::from(settings.padding) * 2);
    let glyph_advance = f32::from(settings.font_size) * 0.62;
    let wrap_columns = ((available_width as f32 / glyph_advance).floor() as usize).max(1);
    let lines = if settings.wrap {
        normalize::wrap_lines(&normalized, wrap_columns)
    } else {
        normalized
            .split('\n')
            .map(|line| clip_line(line, wrap_columns))
            .collect()
    };
    if lines.len() > MAX_LINES {
        bail!(
            "rendered output has more than {MAX_LINES} lines; increase width or reduce the input"
        );
    }

    let line_height = (f32::from(settings.font_size) * 1.55).ceil() as u32;
    let chrome_height = if settings.chrome == ChromeStyle::None {
        0
    } else {
        CHROME_HEIGHT
    };
    let text_height = u64::try_from(lines.len())
        .unwrap_or(u64::MAX)
        .checked_mul(u64::from(line_height))
        .context("render height overflow")?;
    let card_height = u64::from(chrome_height)
        .checked_add(u64::from(settings.padding) * 2)
        .and_then(|value| value.checked_add(text_height))
        .and_then(|value| value.checked_add(12))
        .context("render height overflow")?;
    let height = card_height
        .checked_add(u64::from(OUTER_PADDING) * 2)
        .context("canvas height overflow")?;
    let height = u32::try_from(height).context("render height exceeds supported dimensions")?;

    if height > MAX_DIMENSION || u64::from(width) * u64::from(height) > MAX_PIXELS {
        bail!("requested image is too large ({width}×{height}); reduce the input or font size");
    }

    let svg = build_svg(
        width,
        height,
        card_width,
        card_height as u32,
        &lines,
        theme,
        settings,
    );
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(&svg, &options).context("could not parse generated SVG")?;
    let mut pixmap = Pixmap::new(width, height)
        .context("could not allocate the output image; reduce its dimensions")?;
    resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());
    save_png_atomic(&pixmap, output)?;

    Ok(RenderStats {
        width,
        height,
        lines: lines.len(),
    })
}

fn build_svg(
    width: u32,
    height: u32,
    card_width: u32,
    card_height: u32,
    lines: &[String],
    theme: &Theme,
    settings: &RenderSettings,
) -> String {
    let x = OUTER_PADDING;
    let y = OUTER_PADDING;
    let right = x + card_width;
    let radius = settings.radius;
    let padding = u32::from(settings.padding);
    let font_size = f32::from(settings.font_size);
    let line_height = (font_size * 1.55).ceil();
    let chrome_height = if settings.chrome == ChromeStyle::None {
        0
    } else {
        CHROME_HEIGHT
    };
    let text_x = x + padding;
    let first_baseline = y as f32 + chrome_height as f32 + padding as f32 + font_size * 1.12;
    let text_clip_y = y + chrome_height;
    let text_clip_height = card_height.saturating_sub(chrome_height);
    let mut svg = String::with_capacity(2048 + lines.iter().map(String::len).sum::<usize>() * 2);

    use std::fmt::Write as _;
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\
         <defs>\
         <linearGradient id=\"canvas\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\"><stop offset=\"0%\" stop-color=\"{}\"/><stop offset=\"100%\" stop-color=\"{}\"/></linearGradient>\
         <radialGradient id=\"glow\"><stop offset=\"0%\" stop-color=\"{}\" stop-opacity=\"0.22\"/><stop offset=\"100%\" stop-color=\"{}\" stop-opacity=\"0\"/></radialGradient>\
         <linearGradient id=\"surface\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\"><stop offset=\"0%\" stop-color=\"{}\"/><stop offset=\"100%\" stop-color=\"{}\"/></linearGradient>\
         <clipPath id=\"content-clip\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>\
         </defs>\
         <rect width=\"100%\" height=\"100%\" fill=\"url(#canvas)\"/>\
         <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"url(#glow)\"/>",
        theme.background,
        theme.background_end,
        theme.accent,
        theme.accent,
        theme.surface,
        theme.surface,
        text_x,
        text_clip_y,
        card_width.saturating_sub(padding * 2),
        text_clip_height,
        width / 4,
        y + 40,
        width / 2,
        height.min(480),
    );

    let _ = write!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"#000000\" opacity=\"0.18\"/>\
         <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"#000000\" opacity=\"0.10\"/>\
         <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"url(#surface)\" stroke=\"{}\" stroke-width=\"1\"/>",
        x,
        y + 12,
        card_width,
        card_height,
        radius,
        x,
        y + 6,
        card_width,
        card_height,
        radius,
        x,
        y,
        card_width,
        card_height,
        radius,
        theme.border,
    );

    if chrome_height > 0 {
        let divider_y = y + chrome_height;
        let _ = write!(
            svg,
            "<path d=\"M {} {} H {}\" stroke=\"{}\" stroke-width=\"1\"/>",
            x + u32::from(radius),
            divider_y,
            right - u32::from(radius),
            theme.border,
        );
        match settings.chrome {
            ChromeStyle::Macos => {
                let dot_y = y + chrome_height / 2;
                for (index, color) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
                    let dot_x = x + 24 + index as u32 * 19;
                    let _ = write!(
                        svg,
                        "<circle cx=\"{dot_x}\" cy=\"{dot_y}\" r=\"5.5\" fill=\"{color}\"/>"
                    );
                }
            }
            ChromeStyle::Minimal => {
                let dot_y = y + chrome_height / 2;
                for (index, opacity) in ["0.95", "0.65", "0.35"].iter().enumerate() {
                    let dot_x = x + 24 + index as u32 * 14;
                    let _ = write!(
                        svg,
                        "<circle cx=\"{dot_x}\" cy=\"{dot_y}\" r=\"3.5\" fill=\"{}\" \
                         opacity=\"{opacity}\"/>",
                        theme.muted
                    );
                }
            }
            ChromeStyle::None => {}
        }
        let _ = write!(
            svg,
            "<text x=\"{}\" y=\"{}\" dominant-baseline=\"middle\" font-family=\"Inter, SF Pro Display, Segoe UI, sans-serif\" font-size=\"12\" fill=\"{}\">Terminal</text>\
             <text x=\"{}\" y=\"{}\" text-anchor=\"end\" dominant-baseline=\"middle\" font-family=\"Inter, SF Pro Display, Segoe UI, sans-serif\" font-size=\"10\" font-weight=\"600\" letter-spacing=\"1.6\" fill=\"{}\">PRISM</text>",
            x + 96,
            y + chrome_height / 2,
            theme.muted,
            right - 24,
            y + chrome_height / 2,
            theme.accent,
        );
    }

    let _ = write!(
        svg,
        "<g clip-path=\"url(#content-clip)\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\" xml:space=\"preserve\">",
        FONT_FAMILY, settings.font_size, theme.foreground,
    );
    for (index, line) in lines.iter().enumerate() {
        let baseline = first_baseline + index as f32 * line_height;
        let escaped = escape_xml(line);
        let _ = write!(
            svg,
            "<text x=\"{}\" y=\"{baseline:.2}\">{escaped}</text>",
            text_x
        );
    }
    svg.push_str("</g></svg>");
    svg
}

fn clip_line(line: &str, max_cells: usize) -> String {
    let mut clipped = String::with_capacity(line.len().min(max_cells.saturating_mul(4)));
    let mut width = 0usize;
    for grapheme in line.graphemes(true) {
        let grapheme_width = unicode_width::UnicodeWidthStr::width(grapheme);
        if grapheme_width > 0 && width.saturating_add(grapheme_width) > max_cells {
            break;
        }
        clipped.push_str(grapheme);
        width = width.saturating_add(grapheme_width);
    }
    clipped
}

fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            c if c.is_control() => {}
            c if !is_xml_char(c) => escaped.push('\u{fffd}'),
            c => escaped.push(c),
        }
    }
    escaped
}

fn is_xml_char(ch: char) -> bool {
    matches!(ch as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}

fn save_png_atomic(pixmap: &Pixmap, output: &Path) -> Result<()> {
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("could not create output directory {}", parent.display()))?;

    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .context("output path must have a valid file name")?;
    let temp_file = tempfile::Builder::new()
        .prefix(&format!(".{file_name}."))
        .suffix(".tmp.png")
        .tempfile_in(parent)
        .with_context(|| format!("could not create temporary output in {}", parent.display()))?;
    let temp_path = temp_file.into_temp_path();
    pixmap
        .save_png(&temp_path)
        .with_context(|| format!("could not encode PNG to {}", temp_path.display()))?;
    temp_path
        .persist(output)
        .map_err(|error| error.error)
        .with_context(|| format!("could not move rendered image into {}", output.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::ChromeStyle, theme};
    use std::collections::BTreeMap;

    fn options() -> RenderSettings {
        RenderSettings {
            width: 900,
            font_size: 16,
            padding: 24,
            radius: 16,
            chrome: ChromeStyle::Macos,
            tab_width: 4,
            wrap: true,
        }
    }

    #[test]
    fn svg_xml_escaping_is_safe() {
        assert_eq!(
            escape_xml("<tag attr=\"x\"> & '"),
            "&lt;tag attr=&quot;x&quot;&gt; &amp; &apos;"
        );
    }

    #[test]
    fn renders_a_png_with_expected_signature() {
        let theme = theme::resolve("github-dark", &BTreeMap::new()).unwrap();
        let output =
            std::env::temp_dir().join(format!("prism-render-test-{}.png", std::process::id()));
        let stats = render_png("hello\nworld", &theme, &options(), &output).unwrap();
        let bytes = fs::read(&output).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(stats.width, 900);
        assert_eq!(stats.lines, 2);
        let _ = fs::remove_file(output);
    }

    #[test]
    fn rejects_input_that_becomes_empty_after_normalization() {
        let theme = theme::resolve("github-dark", &BTreeMap::new()).unwrap();
        let output =
            std::env::temp_dir().join(format!("prism-empty-test-{}.png", std::process::id()));
        assert!(render_png("\u{001b}[31m\u{001b}[0m", &theme, &options(), &output).is_err());
        assert!(!output.exists());
    }

    #[test]
    fn invalid_xml_noncharacters_are_replaced() {
        assert_eq!(escape_xml("a\u{fffe}b"), "a\u{fffd}b");
    }

    #[test]
    fn no_wrap_clips_lines_before_svg_generation() {
        assert_eq!(clip_line("abcdef", 3), "abc");
        assert_eq!(clip_line("你好a", 3), "你");
    }

    #[test]
    fn image_dimensions_are_bounded_by_settings_and_line_count() {
        let theme = theme::resolve("github-dark", &BTreeMap::new()).unwrap();
        let content = (0..10_000).map(|_| "line").collect::<Vec<_>>().join("\n");
        let output =
            std::env::temp_dir().join(format!("prism-too-tall-test-{}.png", std::process::id()));
        assert!(render_png(&content, &theme, &options(), &output).is_err());
        let _ = fs::remove_file(output);
    }
}
