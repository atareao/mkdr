use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use clap::CommandFactory;
use clap::Parser;
use clap_complete::{generate, Shell};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod app;
mod config;
mod render;
mod search;
mod theme;

use app::{App, WrapMode};
use config::load_config;
use theme::Theme;

#[derive(Parser)]
#[command(version, about = "TUI markdown renderer with paging",
    long_about = "TUI markdown renderer with paging.

Built-in themes: ayu_dark, ayu_light, ayu_mirage, catppuccin_mocha, dracula, gruvbox_dark, nord, onedark, solarized_light, tokyonight

User themes: place .toml files in ~/.config/mdr/themes/")]
struct Args {
    /// Markdown file(s) to display (reads from stdin if omitted)
    files: Vec<PathBuf>,

    /// Wrap mode: none, word, or char
    #[arg(short = 'w', long = "wrap")]
    wrap: Option<String>,

    /// Show line numbers
    #[arg(short = 'n', long = "line-numbers")]
    line_numbers: bool,

    /// Color theme: auto, light, dark
    #[arg(short = 't', long = "theme", default_value = "ayu_dark")]
    theme: String,

    /// Hide status bar
    #[arg(long = "no-status")]
    no_status: bool,

    /// Start at given line number
    #[arg(short = 'l', long = "line", default_value_t = 1)]
    line: usize,

    /// Follow file changes (watch mode)
    #[arg(short = 'f', long = "follow")]
    follow: bool,

    /// Generate shell completions
    #[arg(long = "completions", value_enum)]
    completions: Option<Shell>,
}

fn main() {
    let cli_args = Args::parse();

    if let Some(shell) = cli_args.completions {
        let mut cmd = Args::command();
        generate(shell, &mut cmd, "mdr", &mut io::stdout());
        return;
    }

    let config = load_config();

    let wrap_mode = {
        let wrap_str = cli_args
            .wrap
            .or_else(|| config.wrap.clone())
            .unwrap_or_else(|| "word".to_string());
        WrapMode::from_str(&wrap_str)
    };
    let line_numbers = cli_args.line_numbers || config.line_numbers.unwrap_or(false);
    let show_status = !cli_args.no_status && config.show_status.unwrap_or(true);

    let theme_name = if cli_args.theme == "auto" {
        config.theme.clone().unwrap_or_else(|| "auto".to_string())
    } else {
        cli_args.theme.clone()
    };

    let theme = match theme_name.as_str() {
        "auto" | "dark" => Theme::default_dark(),
        "light" => Theme::default_light(),
        name => {
            match Theme::load(name) {
                Some(t) => t,
                None => {
                    eprintln!("Warning: theme '{}' not found, using default dark", name);
                    Theme::default_dark()
                }
            }
        }
    };

    let follow = cli_args.follow;

    let stdin_content = if cli_args.files.is_empty() && !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin().lock().read_to_string(&mut buf).ok();
        Some(buf)
    } else {
        None
    };

    if cli_args.files.is_empty() && stdin_content.is_none() {
        eprintln!("Usage: markrender [OPTIONS] <FILE>");
        eprintln!("   or: cat file.md | markrender [OPTIONS]");
        eprintln!();
        eprintln!("Built-in themes: {}", theme::Theme::list_names().join(", "));
        std::process::exit(1);
    }

    // Check files exist before entering raw mode
    for f in &cli_args.files {
        if !f.exists() {
            eprintln!("Error: '{}' not found", f.display());
            eprintln!();
            eprintln!("Usage: markrender [OPTIONS] <FILE>");
            eprintln!("   or: cat file.md | markrender [OPTIONS]");
            eprintln!();
            eprintln!("Built-in themes: {}", theme::Theme::list_names().join(", "));
            std::process::exit(1);
        }
        if f.is_dir() {
            eprintln!("Error: '{}' is a directory", f.display());
            std::process::exit(1);
        }
    }

    enable_raw_mode().expect("failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    terminal.clear().expect("failed to clear terminal");

    let mut app = App::new(
        cli_args.files,
        follow,
        wrap_mode,
        line_numbers,
        show_status,
        theme,
        cli_args.line,
        stdin_content,
    );

    let result = app.run(&mut terminal);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

impl std::str::FromStr for WrapMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(WrapMode::from_str(s))
    }
}