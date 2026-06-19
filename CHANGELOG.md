# Changelog
## [0.6.0] - 2026-05-30

### Bug Fixes

- Preprocess_wiki_links panic on multi-byte UTF-8 characters

### Features

- Add --no-colour, --columns, --local, --fail, --detect-terminal, --ansi CLI options (#23)

### Miscellaneous Tasks

- Release v0.5.3
## [0.5.3] - 2026-05-25

### Bug Fixes

- Remove misleading Release badge (release.yml only runs on tags, shows failing on main)
- Remove misleading Release badge

### Documentation

- Add shields.io badges to README
- Add shields.io badges to README

### Features

- Add Ctrl+u / Ctrl+d half-page scroll shortcuts
- Add Ctrl+u / Ctrl+d half-page scroll shortcuts

### Miscellaneous Tasks

- Release v0.5.3
- Release v0.6.0
- Release v0.6.1
- Release v0.5.3
## [0.5.2] - 2026-05-25

### Bug Fixes

- Inline gh api command to avoid multi-arg parsing issue

### Miscellaneous Tasks

- Release v0.5.2
## [0.5.1] - 2026-05-25

### Miscellaneous Tasks

- Sync development with main after release
- Release v0.5.1
## [0.5.0] - 2026-05-25

### Bug Fixes

- Correct indentation in release.yml workflow and sync Cargo.lock
- Correct indentation in release.yml workflow and sync Cargo.lock

### Miscellaneous Tasks

- Release v0.4.3 (#14)
- Sync development with main (v0.4.3)
- Release v0.5.0

### Other

- Sync development with main (v0.4.2) and resolve conflicts
## [0.4.2] - 2026-05-25

### Miscellaneous Tasks

- Release v0.4.2

### Refactor

- Move insta snapshots from src/snapshots/ to tests/snapshots/ (#11)
## [0.4.1] - 2026-05-25

### Bug Fixes

- Satisfy clippy (collapse if let, alias complex types)
- Replace deprecated cargo publish --token with env var and add --allow-dirty (#12)

### Miscellaneous Tasks

- Misc improvements (CI, docs, Cargo.lock, clippy, fmt)
- Release v0.4.1

### Refactor

- Move insta snapshots to tests/snapshots/ (#13)

### Styling

- Cargo fmt
- Cargo fmt after clippy fixes
## [0.4.0] - 2026-05-25

### Features

- Rename project from mdr to mkdr

### Miscellaneous Tasks

- Miscellaneous improvements
- Release v0.4.0
## [0.3.3] - 2026-05-25

### Bug Fixes

- Rename insta snapshot files from mdr to mkdr
- Add --allow-dirty to cargo publish (Cargo.lock not committed)

### Features

- Rename project from mdr to mkdr

### Miscellaneous Tasks

- Release v0.3.3
## [0.3.2] - 2026-05-25

### Bug Fixes

- Add --allow-dirty to cargo publish (Cargo.lock not committed)
- Remove x86_64-apple-darwin target (macos-13 runners unavailable)

### Miscellaneous Tasks

- Release v0.3.2
## [0.3.1] - 2026-05-24

### Bug Fixes

- Remove x86_64-apple-darwin target (macos-13 runners unavailable)
- Use 7z instead of zip for Windows archive

### Miscellaneous Tasks

- Release v0.3.1
## [0.3.0] - 2026-05-24

### Bug Fixes

- Use GH_PAT token for push to protected main branch
- Use 7z instead of zip for Windows archive (zip not available on Windows runner)
- Use GH_PAT for release push to protected main

### Features

- Prepare release v0.3.0

### Miscellaneous Tasks

- Rename develop branch to development
- Release v0.3.0
- Release v0.3.0
## [0.2.0] - 2026-05-24

### Bug Fixes

- Correct cliff.toml template and vampus subcommand syntax in workflow
- Remove --locked flag from CI build (no Cargo.lock committed)

### Documentation

- Update branch protection details in GIT_FLOW.md

### Features

- Add crates.io publish to release workflow

### Miscellaneous Tasks

- Setup Git Flow with auto versioning and releases
- Add CI workflow for PR checks
- Prepare first automated release
- Rename develop branch to development
- Prepare first automated release (#2)
- Remove release trigger file
- Release v0.2.0
## [0.1.0] - 2026-05-23
