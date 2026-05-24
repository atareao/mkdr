user     := "atareao"
name     := `basename ${PWD}`
version  := `vampus show`


list:
    @just --list

lint:
    cargo clippy --all-targets --all-features

fmt:
    cargo fmt -- --check

fmt-fix:
    cargo fmt

build:
    @podman build \
        --tag={{user}}/{{name}}:{{version}} \
        --tag={{user}}/{{name}}:latest .

push:
    @podman image push {{user}}/{{name}}:{{version}}
    @podman image push {{user}}/{{name}}:latest
tag:
    git tag v{{version}}
    git push origin v{{version}}

upgrade:
    #!/bin/fish
    vampus upgrade --patch
    set VERSION $(vampus show)
    cargo update
    git commit -am "Upgrade to version v$VERSION"
    git tag -a "v$VERSION" -m "Version v$VERSION"
    git push origin v{{version}}
