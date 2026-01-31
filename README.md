<div align="center">

![srapi-rs Banner](https://capsule-render.vercel.app/api?type=waving&color=0:121212,100:333333&height=220&section=header&text=srapi-rs&fontSize=90&fontColor=FFFFFF&animation=fadeIn&fontAlignY=35&rotate=2&stroke=FFFFFF&strokeWidth=2&desc=High-performance%20Client%20for%20Ephemeral%20File%20Hosting&descSize=20&descAlignY=60)


![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?style=for-the-badge)
![Language](https://img.shields.io/badge/language-Rust-orange.svg?style=for-the-badge&logo=rust)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey.svg?style=for-the-badge&logo=linux)

**A unified async library, CLI utility, and FFI binding suite for anonymous file sharing services.**

[Overview](#overview) • [Providers](#supported-providers) • [CLI](#cli-usage) • [Library](#library-integration) • [FFI](#ffi--interop)

</div>

---

> [!IMPORTANT]
> **Unofficial Client**
> This project is an independent client implementation. It is not affiliated with, endorsed by, or associated with Filebin, Temp.sh, Tmpfiles.org, or Jumpshare.
> Please use these services responsibly and ensure compliance with their respective Terms of Service and usage policies.

## Overview

**srapi-rs** abstracts the complexity of interacting with various "pastebin-for-files" providers. Whether you need to quickly share a log file from the command line, integrate anonymous hosting into a Rust application, or bind these capabilities to C/Python via FFI, this project provides a robust solution.

## Key Features

*   **Multi-Provider Support:** Unified access to popular file hosts.
*   **Async Core:** Built on `tokio` and `reqwest` for non-blocking I/O.
*   **Zero-Config CLI:** Ready-to-use command line tool for quick operations.
*   **C ABI (FFI):** Exported shared library for integration with C, C++, Python, etc.

## Supported Providers

| Provider | Type | Features | Expiration | Key Implementation |
| :--- | :--- | :--- | :--- | :--- |
| **[Filebin](https://filebin.net)** | Bucket-based | Bins, File Management, Inspection | ~6 days | `src/backends/filebin.rs` |
| **[Temp.sh](https://temp.sh)** | Direct Upload | Simple Upload, Metadata Inspection | 3 days | `src/backends/temp_sh.rs` |
| **[Tmpfiles.org](https://tmpfiles.org)** | Direct Upload | Simple Upload, Metadata Inspection | 60 mins | `src/backends/tmpfiles.rs` |
| **[Jumpshare](https://jumpshare.com)** | Multipart/S3 | Multipart Upload Only | 1 day | `src/backends/jumpshare.rs` |

## CLI Usage

The CLI supports both stateful (bin-based) and stateless (direct upload) workflows depending on the provider.

### Quick Start

```bash
# Direct upload (Temp.sh, Tmpfiles, Jumpshare)
cargo run -- --provider temp-sh upload-simple ./screenshot.png
cargo run -- --provider jumpshare upload-simple ./video.mp4 --name output.mp4

# Bin management (Filebin)
cargo run -- --provider filebin create-bin
# > returns <BIN_ID>
cargo run -- --provider filebin upload <BIN_ID> ./log.txt
```

### Command Reference

| Command | Description | Supports |
| :--- | :--- | :--- |
| `create-bin` | Initialize a new file collection | Filebin |
| `upload` | Upload file to specific bin | Filebin |
| `details` | List files and metadata in bin | Filebin |
| `delete-file` | Remove specific resource | Filebin |
| `upload-simple` | One-shot upload (returns URL) | Temp.sh, Tmpfiles, Jumpshare |
| `info` | Fetch metadata from URL | Temp.sh, Tmpfiles |

## Library Integration

Add `srapi-rs` to your Cargo configurations. The core trait `FileProvider` handles complex interactions, while `SimpleUploadProvider` handles fire-and-forget uploads.

```rust
use srapi_rs::{FilebinProvider, FileProvider, JumpshareProvider, SimpleUploadProvider};
use tokio::fs::File;

#[tokio::main]
async fn main() {
    // Example: Filebin (Complex)
    let fb = FilebinProvider::new();
    let bin_id = fb.create_bin().await.unwrap();
    println!("Bin created: {}", bin_id);

    // Example: Jumpshare (Simple)
    let js = JumpshareProvider::new();
    let file = tokio::fs::read("test.jpg").await.unwrap();
    let uploaded = js.upload_bytes("test.jpg", file).await.unwrap();
    println!("Direct URL: {}", uploaded.url);
}
```

## FFI & Interop

Compile as a shared library to use `srapi-rs` in other languages.

### Build

```bash
cargo build --release --features ffi
```

Artifact location: `target/release/libsrapi_rs.so` (Linux), `.dylib` (macOS), or `.dll` (Windows).

### Exported Symbols

The library exposes a C-compatible ABI. Memory must be managed explicitly (use the provided `_free` functions).

*   **Constructors:** `srapi_filebin_new`, `srapi_tempsh_new`, `srapi_tmpfiles_new`, `srapi_jumpshare_new`
*   **Destructors:** `srapi_provider_free`, `srapi_string_free`, `srapi_*_free`
*   **Actions:**
    *   `srapi_filebin_create_bin`
    *   `srapi_filebin_upload_file`
    *   `srapi_*_upload_file` (returns URL string)
    *   `srapi_*_get_info` (returns JSON string)

### Python Example

See [tests/test_ffi.py](tests/test_ffi.py) for a complete reference implementation covering all providers.

## Development & Testing

**Requirements:**
*   Rust (latest stable)
*   OpenSSL (pkg-config)

**Running Tests:**
Network tests are live and require an internet connection.

```bash
# Run standard Rust unit/integration tests
cargo test

# Run FFI validation script (requires Python 3)
SRAPI_LIVE_TEST=1 python3 tests/test_ffi.py
```

---

<div align="center">

**Author:** Seuriin ([SSL-ACTX](https://github.com/SSL-ACTX))

*v0.1.0*

</div>
