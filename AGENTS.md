# markrender

TUI markdown renderer with paging (ratatui + termimad).

## Build & run

```bash
cargo build
cargo run -- <file.md>
```

## Edition 2024

`edition = "2024"` — notably changes `unsafe` block hygiene (must qualify `unsafe` on each block) and alters `impl Trait` capture rules. If you see a surprising compile error, check edition migration first: <https://doc.rust-lang.org/nightly/edition-guide/rust-2024/index.html>.

## Toolchain

No `rust-toolchain.toml` — whatever `rustup default` provides. Lockfile **is** committed (`Cargo.lock` in repo) as recommended for applications.

## Dependencies

Add with `cargo add` — do not hand-edit `[dependencies]` unless you have to. Expected key crates:

- `ratatui` — TUI framework
- `pulldown-cmark` — markdown → terminal rendering
- `syntect` — syntax highlighting
- `clap` — CLI arg parsing (markdown file path, optional paging options)

## Testing

Integration tests in `tests/` (snapshot-based with `insta`). Name unit tests `test_*` in a `#[cfg(test)] mod tests { ... }` block in the same file as the code under test. `cargo test` runs all.

## Git Flow

Este proyecto sigue **Git Flow** estricto.

| Rama | Propósito | Base |
|---|---|---|
| `main` | Producción. Cada merge aquí = release automática. | — |
| `development` | Integración de features en curso. | `main` |
| `feature/*` | Nuevas funcionalidades. | `development` |
| `hotfix/*` | Correcciones urgentes a producción. | `main` |

### Conventional Commits con Gitmoji

Formato: `<emoji> <tipo>(<scope>): <mensaje>`

| Tipo | Emoji | Bump |
|---|---|---|
| `feat` | ✨ | minor |
| `feat!` / `BREAKING CHANGE` | 💥 | major |
| `fix` | 🐛 | patch |
| `hotfix` | 🚑️ | patch |
| `refactor` | ♻️ | patch |
| `docs` | 📝 | patch |
| `perf` | ⚡ | patch |
| `test` | ✅ | patch |
| `chore` | 🔧 | patch |
| `ci` | 👷 | patch |

NO hacer version bump manual — CI lo hace automáticamente al mergear a main.