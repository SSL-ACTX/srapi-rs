# Copilot instructions for Filebin (concise)

This file gives targeted, actionable guidance for AI coding agents working in this repository.

Big picture
- Purpose: a small Rust crate exposing a `FileProvider` trait with concrete backends (current: Filebin). See [src/lib.rs](src/lib.rs) for the top-level module layout.
- Key components: the runtime binary (`src/main.rs`) and a library surface (`src/lib.rs`) with backends in [src/backends](src/backends) and core primitives in [src/core](src/core).
- Design intent: provider abstraction (see `FileProvider` in [src/core/provider.rs](src/core/provider.rs)) isolates HTTP scraping/upload logic into backend implementations like [src/backends/filebin.rs](src/backends/filebin.rs).

Build / run / test commands
- Build: `cargo build` (dev) and `cargo build --release` (release).
- Run (example binary): `cargo run --bin srapi-rs` or `cargo run` from project root when invoking `src/main.rs` example flows.
- Tests: `cargo test` (unit/integration). Add tests under `tests/` or inline `#[cfg(test)]` modules in `src/`.

Project-specific conventions
- Provider pattern: implement `FileProvider` in backends and return the crate types `FileMetadata` / `BinInfo` (see [src/core/provider.rs](src/core/provider.rs)).
- Error model: use `ProviderError` (see [src/core/error.rs](src/core/error.rs)) — map `reqwest::Error` and IO errors into this enum.
- HTTP client setup: backends build a `reqwest::Client` with browser-like headers and a 30s timeout (see [src/backends/filebin.rs](src/backends/filebin.rs)). Follow the same pattern when adding new backends.
- Serialization: `rkyv` is used for archive/serialize on `FileMetadata` and `BinInfo` — keep `Archive/Serialize/Deserialize` derives when expanding these structs.

Patterns to follow (do, not guess)
- Streaming uploads: use `tokio` + `tokio_util::codec::BytesCodec` + `reqwest::Body::wrap_stream` as shown in [src/main.rs](src/main.rs).
- Scraping: `Filebin` backend scrapes HTML with `regex`; prefer adding explicit, tested regexes and encapsulate scraping helpers (see `scrape_bin_id`).
- Status handling: treat `404` as `ProviderError::NotFound`; non-success statuses should map to `ProviderError::Api` with the status included.

Integration points & external dependencies
- Network: `reqwest` (rustls TLS) is the primary HTTP client. Set headers consistently via `Client::builder().default_headers(...)`.
- Async runtime: `tokio` multi-threaded runtime; all provider methods are `async` and must be `Send + Sync`.
- Serialization caching: `rkyv` is available for on-disk cache of `BinInfo`/`FileMetadata`.

Small examples (copyable)
- Create & upload (pattern): see [src/main.rs](src/main.rs) for a minimal end-to-end sequence: create bin -> stream file -> call `upload_file` -> call `get_bin_details`.
- Implement backend stub: follow `FilebinProvider::new()` construction and `async_trait` impl in [src/backends/filebin.rs](src/backends/filebin.rs).

Development rules for the AI agent
- Follow strict Test-Driven Development: write a failing unit/integration test first, run `cargo test`, then implement the minimal code to pass.
- Prefer minimal, focused changes in a single commit: update types, add tests, then add backend logic. Don't refactor unrelated modules.
- Use existing types and error variants; add new `ProviderError` variants only when necessary and add corresponding tests.
- When adding network code, include an integration test that can be run locally (mock the HTTP layer with `wiremock` or test on recorded responses). Document any external network test requirements in the test file's doc comment.

Where to look first
- Architecture & rationale: [PLAN.md](PLAN.md) contains notes about the Filebin scraping flow.
- Public API and types: [src/core/provider.rs](src/core/provider.rs) and [src/core/error.rs](src/core/error.rs).
- Existing backend example: [src/backends/filebin.rs](src/backends/filebin.rs).

If something is unclear
- Ask for: the exact test harness expected (unit vs integration), whether network tests are allowed live, and any CI specifics.

After applying changes
- Run `cargo test` locally and include failing/passing output in the PR description.