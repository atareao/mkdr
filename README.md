# mdr

[![release](https://github.com/atareao/mdr/actions/workflows/release.yml/badge.svg)](https://github.com/atareao/mdr/actions/workflows/release.yml)

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
- **Stdin pipe** — `cat file.md | mdr`
- **10 built-in themes** + user themes in `~/.config/mdr/themes/`
- **Config file** at `~/.config/mdr/config.toml`

## Usage

```
mdr [OPTIONS] [FILES]...
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
mdr doc.md
mdr -n -t nord doc.md
cat README.md | mdr
mdr -f -t catppuccin_mocha doc.md
mdr -w none chapter1.md chapter2.md chapter3.md
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
mdr -t nord doc.md
mdr -t ayu_mirage doc.md
```

### User themes

Place `.toml` files in `~/.config/mdr/themes/`:

```toml
# ~/.config/mdr/themes/my_theme.toml
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

`~/.config/mdr/config.toml`:

```toml
wrap = "word"
line_numbers = true
theme = "nord"
show_status = true
```

CLI flags override config values. All fields optional.

## Download

Pre-built binaries for each [release](https://github.com/atareao/mdr/releases):

| Platform | Architecture | File |
|---|---|---|
| Linux | x86_64 | `mdr-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `mdr-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `mdr-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `mdr-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `mdr-x86_64-pc-windows-msvc.zip` |

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

## License

MIT — see [LICENSE](LICENSE).

## Dependencies

[clap](https://crates.io/crates/clap) · [crossterm](https://crates.io/crates/crossterm) · [dirs](https://crates.io/crates/dirs) · [open](https://crates.io/crates/open) · [pulldown-cmark](https://crates.io/crates/pulldown-cmark) · [ratatui](https://crates.io/crates/ratatui) · [serde](https://crates.io/crates/serde) · [toml](https://crates.io/crates/toml) · [unicode-width](https://crates.io/crates/unicode-width)

## Roadmap

- **`clap_complete` as optional dependency** — only used for `--completions`; gate behind a Cargo feature flag
- **Integration tests for navigation** — add tests for keyboard/mode logic (requires mock terminal or extracted state machine)
- **Wiki-link navigation within multi-file sessions** — improve file-history UX (backtrack across multiple hops)
- **Configurable horizontal rule width** — use terminal width instead of fixed 80 columns
- **Blockquote colour theming** — expose quote-depth colours in the theme file instead of hardcoded palette