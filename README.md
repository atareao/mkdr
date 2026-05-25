# mkdr

[![CI](https://img.shields.io/github/actions/workflow/status/atareao/mkdr/ci.yml?style=flat-square&logo=github)](https://github.com/atareao/mkdr/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/actions/workflow/status/atareao/mkdr/release.yml?style=flat-square&logo=github)](https://github.com/atareao/mkdr/actions/workflows/release.yml)
[![Latest](https://img.shields.io/github/v/release/atareao/mkdr?style=flat-square&logo=github)](https://github.com/atareao/mkdr/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-dea584?style=flat-square&logo=rust)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

TUI markdown renderer with paging, search, theming, and multi-file support.

```bash
cargo run -- README.md
```

## Features

- **Markdown rendering** with full theme support (bold, italic, links, tables, code, headings)
- **Interactive links** — `Enter` on a wiki link navigates to the file; `Enter` on a web link opens the browser; `Enter` on an image opens the default image viewer
- **Vim-style navigation** (`j`/`k`, `b`/`f`, `g`/`G`, `←`/`→`)
- **Incremental search** (`/` forward, `?` backward, `n`/`N` next/prev)
- **Go to line** (`:` + number + `Enter`)
- **Word wrap** configurable: `none`, `word`, `char`
- **Line numbers** toggle (`-n`)
- **Multiple files** — pass several files, navigate with `[` / `]`
- **Watch mode** (`-f`) — auto-reloads on file change
- **Stdin pipe** — `cat file.md | mkdr`
- **10 built-in themes** + user themes in `~/.config/mkdr/themes/`
- **Config file** at `~/.config/mkdr/config.toml`

## Usage

```
mkdr [OPTIONS] [FILES]...
```

| Option | Default | Description |
|---|---|---|
| `[FILES]...` | — | Markdown file(s) to display (stdin if omitted) |
| `-w`, `--wrap` | `"word"` | Wrap mode: `none`, `word`, or `char` |
| `-n`, `--line-numbers` | `false` | Show line numbers |
| `-t`, `--theme` | `"ayu_dark"` | Theme: `auto`, `light`, `dark`, or theme name |
| `--no-status` | `false` | Hide status bar |
| `-l`, `--line` | `1` | Start at given line |
| `-f`, `--follow` | `false` | Watch file for changes |

```bash
mkdr doc.md
mkdr -n -t nord doc.md
cat README.md | mkdr
mkdr -f -t catppuccin_mocha doc.md
mkdr -w none chapter1.md chapter2.md chapter3.md
```

## Keybindings

| Key | Action |
|---|---|
| `q` / `Esc` | Quit |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `←` / `→` | Scroll horizontal (when `wrap=none`) |
| `PgUp` / `b` | Page up |
| `PgDn` / `f` | Page down |
| `Ctrl+u` | Half-page up |
| `Ctrl+d` | Half-page down |
| `Home` / `g` | Go to top |
| `End` / `G` | Go to bottom |
| `/` | Search forward |
| `?` | Search backward |
| `n` / `N` | Next / previous match |
| `:` | Go to line |
| `r` | Reload file |
| `[` / `]` | Previous / next file |
| `Enter` | Open link under cursor (wiki → file, web → browser, image → viewer) |

In search and go-to-line modes: `Enter` to confirm, `Esc` to cancel, `Backspace` to delete.

## Themes

10 built-in themes:

`ayu_dark` · `ayu_light` · `ayu_mirage` · `catppuccin_mocha` · `dracula` · `gruvbox_dark` · `nord` · `onedark` · `solarized_light` · `tokyonight`

```bash
mkdr -t nord doc.md
mkdr -t ayu_mirage doc.md
```

### User themes

Place `.toml` files in `~/.config/mkdr/themes/`:

```toml
# ~/.config/mkdr/themes/my_theme.toml
[colors]
text = "#abb2bf"
bg = "#282c34"
blue = "#61afef"

[styles]
paragraph = { fg = "text" }
heading1 = { fg = "blue", bold = true }
inline_code = { fg = "#ce9178", bg = "#2d2d2d" }
```

Style keys: `paragraph`, `bold`, `italic`, `strikeout`, `inline_code`, `code_block`, `link`, `heading1`–`heading6`, `table`, `ellipsis`, `bullet`, `quote_mark`, `horizontal_rule`.

Style fields: `fg` (hex or color reference), `bg`, `bold`, `italic`, `underline`, `strikethrough`.

## Configuration

`~/.config/mkdr/config.toml`:

```toml
wrap = "word"
line_numbers = true
theme = "nord"
show_status = true
```

CLI flags override config values. All fields optional.

## Download

Pre-built binaries for each [release](https://github.com/atareao/mkdr/releases):

| Platform | Architecture | File |
|---|---|---|
| Linux | x86_64 | `mkdr-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `mkdr-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Apple Silicon | `mkdr-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `mkdr-x86_64-pc-windows-msvc.zip` |

Trigger a release by pushing a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The [release workflow](.github/workflows/release.yml) builds all targets, packages them, and creates a GitHub release.

## Build

```bash
cargo build
cargo run -- README.md
```

No `rust-toolchain.toml` — uses whatever `rustup default` provides. Edition 2024.

## Git Flow & releases

See [GIT_FLOW.md](GIT_FLOW.md) for the complete workflow.  
Releases are automatic: merging to `main` triggers CI to bump the version, tag, and publish a GitHub release.

## License

MIT — see [LICENSE](LICENSE).

## Dependencies

[clap](https://crates.io/crates/clap) · [crossterm](https://crates.io/crates/crossterm) · [dirs](https://crates.io/crates/dirs) · [open](https://crates.io/crates/open) · [pulldown-cmark](https://crates.io/crates/pulldown-cmark) · [ratatui](https://crates.io/crates/ratatui) · [serde](https://crates.io/crates/serde) · [toml](https://crates.io/crates/toml) · [unicode-width](https://crates.io/crates/unicode-width)

## Roadmap

- **`clap_complete` as optional dependency** — only used for `--completions`; gate behind a Cargo feature flag
- **Integration tests for navigation** — add tests for keyboard/mode logic (requires mock terminal or extracted state machine)
- **Configurable horizontal rule width** — use terminal width instead of fixed 80 columns
- **Blockquote colour theming** — expose quote-depth colours in the theme file instead of hardcoded palette