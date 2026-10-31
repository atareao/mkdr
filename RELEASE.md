# Release process

See [AGENTS.md](AGENTS.md#git-flow) for the complete Git Flow and release automation.

Releases are automatic: merging `development` into `main` triggers CI to bump the version, generate the changelog, tag, build binaries, create a GitHub Release, and publish to crates.io.