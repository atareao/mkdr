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

No `rust-toolchain.toml` — whatever `rustup default` provides. Lockfile is **not** committed (no `Cargo.lock` in repo). If you need a pinned nightly for ratatui features, add a `rust-toolchain.toml`.

## Dependencies

Add with `cargo add` — do not hand-edit `[dependencies]` unless you have to. Expected key crates:

- `ratatui` — TUI framework
- `termimad` — markdown → terminal rendering
- `clap` — CLI arg parsing (markdown file path, optional paging options)

## Testing

No tests yet. Test structure should follow convention: `cargo test` runs all. Name tests `test_*` in a `#[cfg(test)] mod tests { ... }` block in the same file as the code under test, or in `tests/` for integration tests.

## Git & releases

Git Flow — see [GIT_FLOW.md](GIT_FLOW.md) for full workflow.

- `main` — production (merge aquí = release automático)
- `development` — integración de features
- `feature/*` — ramas de trabajo
- `hotfix/*` — correcciones urgentes desde main

Commits: conventional-commit style (`feat:`, `fix:`, `refactor:`, `chore:`, etc.).
NO hacer version bump manual — CI lo hace automáticamente al mergear a main.