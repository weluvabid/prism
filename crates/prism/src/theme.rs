use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::config::ThemeOverrides;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub background: String,
    pub background_end: String,
    pub surface: String,
    pub foreground: String,
    pub muted: String,
    pub accent: String,
    pub border: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ThemeInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub const BUILT_IN_THEMES: [ThemeInfo; 7] = [
    ThemeInfo {
        id: "monochrome",
        label: "Monochrome",
        description: "Quiet graphite, silver, and soft-white contrast",
    },
    ThemeInfo {
        id: "github-dark",
        label: "GitHub Dark",
        description: "Cool, restrained contrast for code and logs",
    },
    ThemeInfo {
        id: "github-light",
        label: "GitHub Light",
        description: "Clean light surface with blue accents",
    },
    ThemeInfo {
        id: "catppuccin-mocha",
        label: "Catppuccin Mocha",
        description: "Soft, pastel dark palette",
    },
    ThemeInfo {
        id: "nord",
        label: "Nord",
        description: "Arctic blue-gray with quiet contrast",
    },
    ThemeInfo {
        id: "dracula",
        label: "Dracula",
        description: "Deep charcoal with vivid violet accents",
    },
    ThemeInfo {
        id: "rose-pine",
        label: "Rosé Pine",
        description: "Muted, warm color with mauve accents",
    },
];

pub fn resolve(name: &str, configured: &BTreeMap<String, ThemeOverrides>) -> Result<Theme> {
    let requested = normalize_id(name)?;
    let base_theme = built_in(&requested);
    let override_config = configured.get(&requested);

    let mut theme = if let Some(base) = base_theme {
        base
    } else if let Some(custom) = override_config {
        let base_id = custom.extends.as_deref().unwrap_or("github-dark");
        let base_id = normalize_id(base_id)?;
        let Some(base) = built_in(&base_id) else {
            bail!(
                "custom theme `{requested}` must extend one of the built-in themes; \
                 `{base_id}` is not built in"
            );
        };
        base
    } else {
        let available = BUILT_IN_THEMES
            .iter()
            .map(|theme| theme.id)
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "unknown theme `{requested}`. Built-in themes: {available}. \
             Define a custom theme in config.toml to add one."
        );
    };

    theme.name = override_config
        .and_then(|item| item.name.clone())
        .unwrap_or_else(|| {
            BUILT_IN_THEMES
                .iter()
                .find(|item| item.id == requested.as_str())
                .map_or_else(|| title_case(&requested), |item| item.label.to_owned())
        });

    if let Some(overrides) = override_config {
        apply(&mut theme, overrides);
    }
    validate(&mut theme)?;
    Ok(theme)
}

pub fn apply_cli_overrides(
    mut theme: Theme,
    background: Option<&str>,
    foreground: Option<&str>,
    accent: Option<&str>,
) -> Result<Theme> {
    if let Some(value) = background {
        theme.background = value.to_owned();
        theme.background_end = value.to_owned();
    }
    if let Some(value) = foreground {
        theme.foreground = value.to_owned();
    }
    if let Some(value) = accent {
        theme.accent = value.to_owned();
    }
    validate(&mut theme)?;
    Ok(theme)
}

pub fn list(configured: &BTreeMap<String, ThemeOverrides>) -> Vec<(String, String, bool)> {
    let mut items = BUILT_IN_THEMES
        .iter()
        .map(|theme| (theme.id.to_owned(), theme.label.to_owned(), false))
        .collect::<Vec<_>>();
    for (id, custom) in configured {
        if !BUILT_IN_THEMES.iter().any(|theme| theme.id == id.as_str()) {
            items.push((
                id.clone(),
                custom.name.clone().unwrap_or_else(|| title_case(id)),
                true,
            ));
        }
    }
    items.sort_by(|left, right| left.0.cmp(&right.0));
    items
}

fn normalize_id(name: &str) -> Result<String> {
    let id = name.trim().to_ascii_lowercase();
    if id.is_empty()
        || !id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        bail!("theme names may contain only lowercase letters, digits, and hyphens");
    }
    Ok(id)
}

fn apply(theme: &mut Theme, overrides: &ThemeOverrides) {
    if let Some(value) = &overrides.background {
        theme.background.clone_from(value);
    }
    if let Some(value) = &overrides.background_end {
        theme.background_end.clone_from(value);
    }
    if let Some(value) = &overrides.surface {
        theme.surface.clone_from(value);
    }
    if let Some(value) = &overrides.foreground {
        theme.foreground.clone_from(value);
    }
    if let Some(value) = &overrides.muted {
        theme.muted.clone_from(value);
    }
    if let Some(value) = &overrides.accent {
        theme.accent.clone_from(value);
    }
    if let Some(value) = &overrides.border {
        theme.border.clone_from(value);
    }
}

fn validate(theme: &mut Theme) -> Result<()> {
    theme.background = normalize_color("background", &theme.background)?;
    theme.background_end = normalize_color("background_end", &theme.background_end)?;
    theme.surface = normalize_color("surface", &theme.surface)?;
    theme.foreground = normalize_color("foreground", &theme.foreground)?;
    theme.muted = normalize_color("muted", &theme.muted)?;
    theme.accent = normalize_color("accent", &theme.accent)?;
    theme.border = normalize_color("border", &theme.border)?;
    Ok(())
}

fn normalize_color(field: &str, color: &str) -> Result<String> {
    let value = color.trim();
    let hex = value.strip_prefix('#').unwrap_or("");
    let expanded = match hex.len() {
        3 if hex.chars().all(|ch| ch.is_ascii_hexdigit()) => {
            let mut output = String::with_capacity(6);
            for ch in hex.chars() {
                output.push(ch);
                output.push(ch);
            }
            output
        }
        6 if hex.chars().all(|ch| ch.is_ascii_hexdigit()) => hex.to_owned(),
        _ => bail!("`{field}` must be a hex color such as `#7aa2f7` or `#abc`"),
    };
    Ok(format!("#{expanded}").to_ascii_lowercase())
}

fn title_case(id: &str) -> String {
    id.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn built_in(id: &str) -> Option<Theme> {
    let theme = match id {
        "monochrome" => Theme {
            name: "Monochrome".to_owned(),
            background: "#171719".to_owned(),
            background_end: "#222225".to_owned(),
            surface: "#1d1d20".to_owned(),
            foreground: "#e8e8ea".to_owned(),
            muted: "#a0a0a6".to_owned(),
            accent: "#c4c4ca".to_owned(),
            border: "#38383e".to_owned(),
        },
        "github-dark" => Theme {
            name: "GitHub Dark".to_owned(),
            background: "#0b1020".to_owned(),
            background_end: "#172238".to_owned(),
            surface: "#0d1117".to_owned(),
            foreground: "#e6edf3".to_owned(),
            muted: "#8b949e".to_owned(),
            accent: "#58a6ff".to_owned(),
            border: "#30363d".to_owned(),
        },
        "github-light" => Theme {
            name: "GitHub Light".to_owned(),
            background: "#eef3fa".to_owned(),
            background_end: "#dce6f3".to_owned(),
            surface: "#ffffff".to_owned(),
            foreground: "#24292f".to_owned(),
            muted: "#57606a".to_owned(),
            accent: "#0969da".to_owned(),
            border: "#d0d7de".to_owned(),
        },
        "catppuccin-mocha" => Theme {
            name: "Catppuccin Mocha".to_owned(),
            background: "#11111b".to_owned(),
            background_end: "#24243a".to_owned(),
            surface: "#1e1e2e".to_owned(),
            foreground: "#cdd6f4".to_owned(),
            muted: "#a6adc8".to_owned(),
            accent: "#cba6f7".to_owned(),
            border: "#45475a".to_owned(),
        },
        "nord" => Theme {
            name: "Nord".to_owned(),
            background: "#242933".to_owned(),
            background_end: "#3b4252".to_owned(),
            surface: "#2e3440".to_owned(),
            foreground: "#eceff4".to_owned(),
            muted: "#d8dee9".to_owned(),
            accent: "#88c0d0".to_owned(),
            border: "#4c566a".to_owned(),
        },
        "dracula" => Theme {
            name: "Dracula".to_owned(),
            background: "#191a21".to_owned(),
            background_end: "#303342".to_owned(),
            surface: "#282a36".to_owned(),
            foreground: "#f8f8f2".to_owned(),
            muted: "#bdc0d0".to_owned(),
            accent: "#bd93f9".to_owned(),
            border: "#44475a".to_owned(),
        },
        "rose-pine" => Theme {
            name: "Rosé Pine".to_owned(),
            background: "#191724".to_owned(),
            background_end: "#26233a".to_owned(),
            surface: "#1f1d2e".to_owned(),
            foreground: "#e0def4".to_owned(),
            muted: "#908caa".to_owned(),
            accent: "#c4a7e7".to_owned(),
            border: "#403d52".to_owned(),
        },
        _ => return None,
    };
    Some(theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monochrome_palette_uses_neutral_colors() {
        let theme = resolve("monochrome", &BTreeMap::new()).unwrap();
        for color in [
            theme.background,
            theme.background_end,
            theme.surface,
            theme.foreground,
            theme.muted,
            theme.accent,
            theme.border,
        ] {
            let rgb = color.trim_start_matches('#');
            let channels = [
                u8::from_str_radix(&rgb[0..2], 16).unwrap(),
                u8::from_str_radix(&rgb[2..4], 16).unwrap(),
                u8::from_str_radix(&rgb[4..6], 16).unwrap(),
            ];
            assert_eq!(channels[0], channels[1], "{color} is not neutral");
            assert_eq!(channels[1], channels[2], "{color} is not neutral");
        }
    }

    #[test]
    fn all_built_in_palettes_are_valid() {
        for info in BUILT_IN_THEMES {
            assert!(resolve(info.id, &BTreeMap::new()).is_ok(), "{}", info.id);
        }
    }

    #[test]
    fn custom_theme_inherits_builtin_values() {
        let mut configured = BTreeMap::new();
        configured.insert(
            "my-theme".to_owned(),
            ThemeOverrides {
                extends: Some("nord".to_owned()),
                name: Some("My Theme".to_owned()),
                accent: Some("#abc".to_owned()),
                ..ThemeOverrides::default()
            },
        );
        let theme = resolve("my-theme", &configured).unwrap();
        assert_eq!(theme.background, "#242933");
        assert_eq!(theme.accent, "#aabbcc");
        assert_eq!(theme.name, "My Theme");
    }

    #[test]
    fn rejects_invalid_colors() {
        let configured = BTreeMap::from([(
            "bad".to_owned(),
            ThemeOverrides {
                accent: Some("url(javascript:alert(1))".to_owned()),
                ..ThemeOverrides::default()
            },
        )]);
        assert!(resolve("bad", &configured).is_err());
    }

    #[test]
    fn custom_theme_cannot_extend_another_custom_theme() {
        let configured = BTreeMap::from([
            ("base-custom".to_owned(), ThemeOverrides::default()),
            (
                "derived-custom".to_owned(),
                ThemeOverrides {
                    extends: Some("base-custom".to_owned()),
                    ..ThemeOverrides::default()
                },
            ),
        ]);
        assert!(resolve("derived-custom", &configured).is_err());
    }
}
