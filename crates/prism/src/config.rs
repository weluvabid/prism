use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub const CONFIG_LIMIT_BYTES: u64 = 1024 * 1024;
pub const DEFAULT_MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const HARD_MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChromeStyle {
    #[default]
    Macos,
    Minimal,
    None,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub defaults: Defaults,
    pub themes: BTreeMap<String, ThemeOverrides>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
    pub theme: String,
    pub width: u32,
    pub font_size: u16,
    pub padding: u16,
    pub radius: u16,
    pub chrome: ChromeStyle,
    pub tab_width: usize,
    pub wrap: bool,
    pub max_input_bytes: usize,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            theme: "monochrome".to_owned(),
            width: 1200,
            font_size: 16,
            padding: 30,
            radius: 18,
            chrome: ChromeStyle::Macos,
            tab_width: 4,
            wrap: true,
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeOverrides {
    pub extends: Option<String>,
    pub name: Option<String>,
    pub background: Option<String>,
    pub background_end: Option<String>,
    pub surface: Option<String>,
    pub foreground: Option<String>,
    pub muted: Option<String>,
    pub accent: Option<String>,
    pub border: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RenderSettings {
    pub width: u32,
    pub font_size: u16,
    pub padding: u16,
    pub radius: u16,
    pub chrome: ChromeStyle,
    pub tab_width: usize,
    pub wrap: bool,
}

impl RenderSettings {
    pub fn validate(&self) -> Result<()> {
        if !(640..=4096).contains(&self.width) {
            bail!("width must be between 640 and 4096 pixels");
        }
        if !(10..=32).contains(&self.font_size) {
            bail!("font size must be between 10 and 32 pixels");
        }
        if !(8..=128).contains(&self.padding) {
            bail!("padding must be between 8 and 128 pixels");
        }
        if !(4..=48).contains(&self.radius) {
            bail!("corner radius must be between 4 and 48 pixels");
        }
        if !(1..=16).contains(&self.tab_width) {
            bail!("tab width must be between 1 and 16 columns");
        }
        Ok(())
    }
}

pub fn config_path() -> Result<PathBuf> {
    let root = dirs::config_dir().context("could not determine the platform config directory")?;
    Ok(root.join("prism").join("config.toml"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }

    let metadata = fs::metadata(&path)
        .with_context(|| format!("could not inspect config file {}", path.display()))?;
    if metadata.len() > CONFIG_LIMIT_BYTES {
        bail!(
            "config file is larger than the 1 MiB limit: {}",
            path.display()
        );
    }

    let source = fs::read_to_string(&path)
        .with_context(|| format!("could not read config file {}", path.display()))?;
    toml::from_str(&source)
        .with_context(|| format!("could not parse config file {}", path.display()))
}

pub fn template() -> &'static str {
    r##"# Prism configuration. CLI flags override these defaults.

[defaults]
theme = "monochrome"
width = 1200
font_size = 16
padding = 30
radius = 18
chrome = "macos"
tab_width = 4
wrap = true
max_input_bytes = 8388608

# Custom themes inherit one built-in palette, then override any fields.
# Available bases: monochrome, github-dark, github-light, catppuccin-mocha, nord, dracula, rose-pine.
# [themes.my-theme]
# extends = "nord"
# name = "My Theme"
# background = "#101820"
# background_end = "#192a37"
# surface = "#101820"
# foreground = "#e6edf3"
# muted = "#9aa9b8"
# accent = "#7aa2f7"
# border = "#314253"
"##
}

pub fn initialize(force: bool) -> Result<PathBuf> {
    let path = config_path()?;
    if path.exists() && !force {
        bail!(
            "config already exists at {}; use `prism config init --force` to replace it",
            path.display()
        );
    }
    let parent = path
        .parent()
        .context("config path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("could not create config directory {}", parent.display()))?;
    fs::write(&path, template())
        .with_context(|| format!("could not write config file {}", path.display()))?;
    Ok(path)
}

pub fn show() -> Result<()> {
    let path = config_path()?;
    println!("{}", path.display());
    if path.exists() {
        let source = fs::read_to_string(&path)
            .with_context(|| format!("could not read config file {}", path.display()))?;
        print!("{source}");
    } else {
        println!("\nNo config file exists yet. Template:\n");
        print!("{}", template());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_complete_when_only_one_key_is_set() {
        let config: Config = toml::from_str("[defaults]\ntheme = 'nord'\n").unwrap();
        assert_eq!(config.defaults.theme, "nord");
        assert_eq!(config.defaults.width, 1200);
        assert_eq!(config.defaults.tab_width, 4);
        assert!(config.defaults.wrap);
    }

    #[test]
    fn render_settings_reject_excessive_dimensions() {
        let settings = RenderSettings {
            width: 9000,
            font_size: 16,
            padding: 30,
            radius: 18,
            chrome: ChromeStyle::Macos,
            tab_width: 4,
            wrap: true,
        };
        assert!(settings.validate().is_err());
    }
}
