use std::{path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use prism::{
    config::{self, ChromeStyle, Config, HARD_MAX_INPUT_BYTES, RenderSettings},
    input, render, theme,
};

#[derive(Debug, Parser)]
#[command(
    name = "prism",
    version,
    about = "Turn supplied terminal text into polished, shareable PNGs"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, value_name = "PATH", conflicts_with = "clipboard")]
    input: Option<PathBuf>,

    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    #[arg(long, value_name = "THEME")]
    theme: Option<String>,

    #[arg(long = "bg", value_name = "HEX")]
    background: Option<String>,

    #[arg(long = "fg", value_name = "HEX")]
    foreground: Option<String>,

    #[arg(long, value_name = "HEX")]
    accent: Option<String>,

    #[arg(long, value_name = "PIXELS")]
    width: Option<u32>,

    #[arg(long = "font-size", value_name = "PIXELS")]
    font_size: Option<u16>,

    #[arg(long, value_name = "PIXELS")]
    padding: Option<u16>,

    #[arg(long, value_name = "PIXELS")]
    radius: Option<u16>,

    #[arg(long, value_enum)]
    chrome: Option<ChromeStyle>,

    #[arg(long, value_name = "COLUMNS")]
    tab_width: Option<usize>,

    #[arg(long, conflicts_with = "input")]
    clipboard: bool,

    #[arg(long, conflicts_with = "no_wrap")]
    wrap: bool,

    #[arg(long, conflicts_with = "wrap")]
    no_wrap: bool,

    #[arg(long, value_name = "BYTES")]
    max_input_bytes: Option<usize>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List built-in and configured themes.
    Themes,
    /// Inspect or initialize the user configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Print the config path and its contents.
    Show,
    /// Create a starter config file.
    Init {
        /// Replace an existing config file.
        #[arg(long)]
        force: bool,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command.as_ref() {
        Some(Command::Config {
            command: ConfigCommand::Show,
        }) => return config::show(),
        Some(Command::Config {
            command: ConfigCommand::Init { force },
        }) => {
            let path = config::initialize(*force)?;
            println!("Created {}", path.display());
            return Ok(());
        }
        _ => {}
    }

    let config = config::load()?;
    if matches!(cli.command.as_ref(), Some(Command::Themes)) {
        print_themes(&config);
        return Ok(());
    }

    create_image(&cli, &config)
}

fn create_image(cli: &Cli, config: &Config) -> Result<()> {
    let defaults = &config.defaults;
    let max_input_bytes = cli.max_input_bytes.unwrap_or(defaults.max_input_bytes);
    if max_input_bytes == 0 || max_input_bytes > HARD_MAX_INPUT_BYTES {
        bail!("--max-input-bytes must be between 1 and {HARD_MAX_INPUT_BYTES}");
    }

    let input_text = input::read_input(cli.input.as_deref(), cli.clipboard, max_input_bytes)?;
    let requested_theme = cli.theme.as_deref().unwrap_or(defaults.theme.as_str());
    let palette = theme::resolve(requested_theme, &config.themes)?;
    let palette = theme::apply_cli_overrides(
        palette,
        cli.background.as_deref(),
        cli.foreground.as_deref(),
        cli.accent.as_deref(),
    )?;
    let settings = RenderSettings {
        width: cli.width.unwrap_or(defaults.width),
        font_size: cli.font_size.unwrap_or(defaults.font_size),
        padding: cli.padding.unwrap_or(defaults.padding),
        radius: cli.radius.unwrap_or(defaults.radius),
        chrome: cli.chrome.unwrap_or(defaults.chrome),
        tab_width: cli.tab_width.unwrap_or(defaults.tab_width),
        wrap: if cli.no_wrap {
            false
        } else if cli.wrap {
            true
        } else {
            defaults.wrap
        },
    };
    settings.validate()?;

    let output = cli
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("prism.png"));
    let stats = render::render_png(&input_text, &palette, &settings, &output)
        .with_context(|| format!("could not render theme `{requested_theme}`"))?;
    println!(
        "Wrote {} ({}×{}, {} lines, {})",
        output.display(),
        stats.width,
        stats.height,
        stats.lines,
        palette.name
    );
    Ok(())
}

fn print_themes(config: &Config) {
    println!("Available themes:\n");
    for (id, label, custom) in theme::list(&config.themes) {
        let marker = if custom { " (custom)" } else { "" };
        let description = theme::BUILT_IN_THEMES
            .iter()
            .find(|info| info.id == id.as_str())
            .map_or("User-defined palette", |info| info.description);
        println!("  {id:<20} {label}{marker}\n  {description}\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_basic_render_options() {
        let cli = Cli::try_parse_from([
            "prism",
            "--theme",
            "nord",
            "--bg",
            "#101820",
            "--width",
            "1440",
            "--no-wrap",
        ])
        .unwrap();
        assert_eq!(cli.theme.as_deref(), Some("nord"));
        assert_eq!(cli.background.as_deref(), Some("#101820"));
        assert_eq!(cli.width, Some(1440));
        assert!(cli.no_wrap);
    }

    #[test]
    fn clipboard_and_file_inputs_are_mutually_exclusive() {
        assert!(Cli::try_parse_from(["prism", "--clipboard", "--input", "output.txt"]).is_err());
    }
}
