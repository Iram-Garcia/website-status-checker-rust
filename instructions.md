# Final Project Assignment: Website Status Checker (Rust)

## 1. Project Overview

Design and implement a concurrent website-monitoring tool that can check the availability of many websites in parallel. The finished program will be a command-line utility written entirely in Rust.

**Purpose:** Reinforce your understanding of Rust threads, channels, error handling, external-process management, and simple CLI design.

---

## 2. Learning Goals

| Concept                             | Demonstrated By                                                                                           |
|-------------------------------------|------------------------------------------------------------------------------------------------------------|
| Thread creation & coordination       | Fixed worker-thread pool that pulls jobs from a channel                                                     |
| Ownership & borrowing across threads | Sharing configuration and sending data structures safely                                                   |
| Error handling                      | Returning `Result<_, String>` without panics                                                               |
| Measuring latency                   | Using `std::time::Instant`                                                                                 |
| Simple JSON generation              | Building a JSON file manually, no helper crates                                                            |
| External crate integration          | Using a single HTTP client crate (`reqwest`) correctly                                                      |

---

## 3. Functional Requirements

### 3.1 Input Sources

- **File:** `--file <path>` (text file, one URL per line; blank lines & lines starting with `#` are ignored).
- **Positional arguments:** Any URLs given directly after flags.

### 3.2 Concurrency

- A fixed pool of worker threads (`--workers N`, default = number of logical CPU cores).

### 3.3 Timeout

- Per-request timeout (`--timeout <seconds>`, default = 5 s) enforced via the HTTP client builder.

### 3.4 Retries

- Optional `--retries <N>` (default = 0) additional attempts after a failure, with a 100 ms pause between attempts.

### 3.5 Result Capture

- For each URL collect:
  - HTTP status code or error string.
  - Total response time.
  - A timestamp.

### 3.6 Live Output

- Immediately print one human-readable line per URL to stdout.

### 3.7 Batch Output

- After all URLs finish, write `status.json` containing an array of objects with the same data.

---

## 4. WebsiteStatus Structure

```rust
struct WebsiteStatus {
    url: String,                 // original URL
    action_status: Result<u16, String>, // HTTP code or error text
    response_time: Duration,     // how long the request took
    timestamp: SystemTime        // when the attempt completed
}
