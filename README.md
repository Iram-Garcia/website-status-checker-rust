# website-status-checker-rust

## Introduction

`website-status-checker-rust` is a final project for CSCI-3334 Systems Programming.

## Build

In order to build the project we begin with the following commands in the terminal

```bash
# Clone the repo
git clone https://github.com/iraml/website-status-checker-rust.git
cd website-status-checker-rust

# Build in release mode
cargo build --release
```

## Usage

In order to use and run the program, there are several options:

```bash
# Single URL
cargo run --release -- https://example.com

# Multiple URLs
cargo run --release -- https://example.com https://rust-lang.org

# .txt file 
cargo run -- --file "file path\URL.txt"
```

## How It Works

As an overview of how the program works, it can be broken down into the following steps:

1. **Argument parsing**: Uses `clap` to accept URLs and optional flags (e.g., timeouts, concurrency).
2. **Request dispatch**: Spawns an asynchronous task per URL to send HTTP GET via `reqwest`.
3. **Result collection**: Gathers each response’s status code or error and prints a summary.

## API Reference

### Struct used

- **RequestResult**
  - `url: String` – the target URL
  - `status: Option<StatusCode>` – HTTP status code if the request succeeded
  - `error: Option<String>` – error message if the request failed

### Functions used

- **check_url_status(url: &str) -> RequestResult**  
  Sends an async GET request to `url` and returns a `RequestResult` containing either the status code or error.

- **run_checker(urls: Vec<String>, concurrency: usize)**  
  Orchestrates spawning tasks up to `concurrency`, waits for all to finish, and outputs results.

- **main()**  
  Parses CLI arguments, invokes `run_checker`, and handles exit codes.