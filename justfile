#!/usr/bin/env just --justfile

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

release:
  cargo build --release

lint:
  cargo clippy

publish: release
  cargo publish

build:
    cargo build

server:
    target/debug/synclip server 5505

client:
    target/debug/synclip client http://localhost:5505

win:
    cargo build --target=x86_64-pc-windows-gnu

publish-win: win
    scp target/x86_64-pc-windows-gnu/debug/synclip.exe win11:d:\\bppleman\\synclip\\target\\debug\\synclip.exe

linux:
    cargo build --target=x86_64-unknown-linux-gnu

publish-linux: linux
    scp target/x86_64-unknown-linux-gnu/debug/synclip ubuntu:/home/bppleman/CLionProjects/synclip/target/debug/synclip
