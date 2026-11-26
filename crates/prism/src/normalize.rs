use anyhow::{Result, bail};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    Csi,
    Osc,
    OscEscape,
    StringControl,
    StringEscape,
    Charset,
}

pub const MAX_LINES: usize = 10_000;

pub fn normalize(input: &str, tab_width: usize, max_output_bytes: usize) -> Result<String> {
    if max_output_bytes == 0 {
        bail!("normalized text limit must be greater than zero");
    }
    let tab_width = tab_width.clamp(1, 16);
    let mut output = String::with_capacity(input.len().min(max_output_bytes).min(8 * 1024 * 1024));
    let mut line_cells: Vec<u8> = Vec::new();
    let mut cell_width = 0usize;
    let mut state = State::Ground;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match state {
            State::Ground => match ch {
                '\u{001b}' => state = State::Escape,
                '\u{009b}' => state = State::Csi,
                '\u{009d}' => state = State::Osc,
                '\u{0090}' | '\u{0098}' | '\u{009e}' | '\u{009f}' => {
                    state = State::StringControl;
                }
                '\u{009c}' => {}
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next().expect("peeked newline");
                        let limit = if chars.peek().is_none() {
                            max_output_bytes.saturating_add(1)
                        } else {
                            max_output_bytes
                        };
                        push_char(&mut output, '\n', limit)?;
                        line_cells.clear();
                        cell_width = 0;
                    } else {
                        truncate_current_line(&mut output);
                        line_cells.clear();
                        cell_width = 0;
                    }
                }
                '\n' => {
                    let limit = if chars.peek().is_none() {
                        max_output_bytes.saturating_add(1)
                    } else {
                        max_output_bytes
                    };
                    push_char(&mut output, '\n', limit)?;
                    line_cells.clear();
                    cell_width = 0;
                }
                '\t' => {
                    let spaces = tab_width - (cell_width % tab_width);
                    if output.len().saturating_add(spaces) > max_output_bytes {
                        bail!(
                            "normalized text exceeds the configured {max_output_bytes} byte limit"
                        );
                    }
                    output.extend(std::iter::repeat_n(' ', spaces));
                    line_cells.extend(std::iter::repeat_n(1_u8, spaces));
                    cell_width += spaces;
                }
                '\u{0008}' => {
                    if output
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous != '\n')
                    {
                        output.pop().expect("backspace checked nonempty output");
                        if let Some(width) = line_cells.pop() {
                            cell_width = cell_width.saturating_sub(usize::from(width));
                        }
                    }
                }
                c if is_c1_control(c) || c.is_control() => {}
                c => {
                    push_char(&mut output, c, max_output_bytes)?;
                    let width = UnicodeWidthChar::width(c).unwrap_or(0);
                    line_cells.push(width.min(255) as u8);
                    cell_width = cell_width.saturating_add(width);
                }
            },
            State::Escape => {
                state = match ch {
                    '[' => State::Csi,
                    ']' => State::Osc,
                    'P' | 'X' | '^' | '_' => State::StringControl,
                    '(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' | '%' | '&' | '!' => {
                        State::Charset
                    }
                    '\u{001b}' => State::Escape,
                    _ => State::Ground,
                };
            }
            State::Csi => {
                if ('\u{0040}'..='\u{007e}').contains(&ch) {
                    state = State::Ground;
                } else if ch == '\u{001b}' {
                    state = State::Escape;
                }
            }
            State::Osc => match ch {
                '\u{0007}' | '\u{009c}' => state = State::Ground,
                '\u{001b}' => state = State::OscEscape,
                _ => {}
            },
            State::OscEscape => match ch {
                '\\' | '\u{009c}' => state = State::Ground,
                '\u{001b}' => state = State::OscEscape,
                _ => state = State::Osc,
            },
            State::StringControl => match ch {
                '\u{009c}' => state = State::Ground,
                '\u{001b}' => state = State::StringEscape,
                _ => {}
            },
            State::StringEscape => match ch {
                '\\' | '\u{009c}' => state = State::Ground,
                '\u{001b}' => state = State::StringEscape,
                _ => state = State::StringControl,
            },
            State::Charset => state = State::Ground,
        }
    }

    if output.ends_with('\n') {
        output.pop().expect("ends_with checked final newline");
    }
    Ok(output)
}

fn push_char(output: &mut String, ch: char, max_output_bytes: usize) -> Result<()> {
    if output.len().saturating_add(ch.len_utf8()) > max_output_bytes {
        bail!("normalized text exceeds the configured {max_output_bytes} byte limit");
    }
    output.push(ch);
    Ok(())
}

fn is_c1_control(ch: char) -> bool {
    ('\u{0080}'..='\u{009f}').contains(&ch)
}

fn truncate_current_line(output: &mut String) {
    if let Some(index) = output.rfind('\n') {
        output.truncate(index + 1);
    } else {
        output.clear();
    }
}

pub fn wrap_lines(content: &str, max_cells: usize) -> Vec<String> {
    let max_cells = max_cells.max(1);
    let mut wrapped = Vec::new();

    for line in content.split('\n') {
        let mut current = String::new();
        let mut width = 0usize;
        for grapheme in line.graphemes(true) {
            let grapheme_width = UnicodeWidthStr::width(grapheme);
            if !current.is_empty()
                && grapheme_width > 0
                && width.saturating_add(grapheme_width) > max_cells
            {
                wrapped.push(std::mem::take(&mut current));
                if wrapped.len() > MAX_LINES {
                    return wrapped;
                }
                width = 0;
            }
            current.push_str(grapheme);
            width = width.saturating_add(grapheme_width);
        }
        wrapped.push(current);
        if wrapped.len() > MAX_LINES {
            break;
        }
    }

    wrapped
}

pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_sgr_and_terminal_control_sequences() {
        let source = "\u{001b}[31mred\u{001b}[0m \u{001b}[2Jdone";
        assert_eq!(normalize(source, 4, 1024).unwrap(), "red done");
    }

    #[test]
    fn strips_osc_titles_and_links() {
        let source = concat!(
            "\u{001b}]0;secret title\u{0007}visible",
            "\u{001b}]8;;https://example.com\u{001b}\\link",
            "\u{001b}]8;;\u{001b}\\"
        );
        assert_eq!(normalize(source, 4, 1024).unwrap(), "visiblelink");
    }

    #[test]
    fn handles_progress_carriage_returns() {
        assert_eq!(normalize("10%\r20%\r100%\n", 4, 1024).unwrap(), "100%");
        assert_eq!(
            normalize("first\r\nsecond\r\n", 4, 1024).unwrap(),
            "first\nsecond"
        );
    }

    #[test]
    fn expands_tabs_from_current_display_column() {
        assert_eq!(normalize("a\tb\n\tend", 4, 1024).unwrap(), "a   b\n    end");
    }

    #[test]
    fn backspace_removes_the_previous_scalar() {
        assert_eq!(normalize("hello\u{0008}!", 4, 1024).unwrap(), "hell!");
    }

    #[test]
    fn preserves_unicode_and_indentation() {
        assert_eq!(
            normalize("  你好\n    ok", 4, 1024).unwrap(),
            "  你好\n    ok"
        );
    }

    #[test]
    fn wraps_using_terminal_cell_width() {
        assert_eq!(
            wrap_lines("ab你好cd", 6),
            vec!["ab你好".to_owned(), "cd".to_owned()]
        );
        assert_eq!(display_width("ab你好"), 6);
    }

    #[test]
    fn wrapping_does_not_split_grapheme_clusters() {
        let family = "👨‍👩‍👧‍👦";
        assert_eq!(
            wrap_lines(&format!("{family}x"), 2),
            vec![family.to_owned(), "x".to_owned()]
        );
    }

    #[test]
    fn normalized_output_is_bounded_without_rejecting_a_trimmed_final_newline() {
        assert!(normalize("\t\t\t", 16, 32).is_err());
        assert_eq!(normalize("\t", 4, 4).unwrap(), "    ");
        assert_eq!(normalize("abcd\n", 4, 4).unwrap(), "abcd");
    }
}
