#!/usr/bin/env just --justfile

release:
  cargo build --release

lint:
  cargo clippy

publish: release
  cargo publish
