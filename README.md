# synclip

A cross-network clipboard synchronization tool.

# Quick Start

## Installation

```bash
cargo install synclip
```

## Usage

* Start the server

```bash
# Specify the port to listen on
synclip server 5505
```

* Start the client

```bash
# Like http://[server]:[port]
synclip client http://localhost:5505
```

Then you can copy text on one computer and paste it on another.
