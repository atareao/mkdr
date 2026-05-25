# Release process

See [GIT_FLOW.md](GIT_FLOW.md) for the complete Git Flow and release automation.

Releases are automatic: merging `development` into `main` triggers CI to bump the version, generate the changelog, tag, build binaries, create a GitHub Release, and publish to crates.io.