# TODO

All originally planned features have been implemented.

## Future ideas

- **`clap_complete` as optional dependency** — only used for `--completions`; gate behind a Cargo feature flag
- **Integration tests for navigation** — add tests for keyboard/mode logic (requires mock terminal or extracted state machine)
- **Configurable horizontal rule width** — use terminal width instead of fixed 80 columns
- **Blockquote colour theming** — expose quote-depth colours in the theme file instead of hardcoded palette