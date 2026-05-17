# Task 2: Define Domain Models And Errors

## Plan

- [x] Create RED tests for downloader domain models and structured errors.
- [x] Confirm the RED tests fail for missing implementation.
- [x] Implement `src-tauri/src/models.rs` with serializable domain models.
- [x] Implement `src-tauri/src/errors.rs` with frontend-safe structured errors.
- [x] Export modules from `src-tauri/src/lib.rs` while preserving `run()`.
- [x] Run verification commands:
  - `cargo test models` and `cargo test errors`
  - `cargo check`
  - `git diff --check`
- [x] Commit with message `feat: add downloader domain models`.

## Review

RED note: `cargo test models errors` is invalid Cargo syntax because Cargo accepts a single test filter. Use separate model and error test filters instead.

Verification completed:

- `cargo test models`: 2 passed.
- `cargo test errors`: 1 passed.
- `cargo test`: 3 passed.
- `cargo check`: passed.
- `cargo fmt -- --check`: passed after formatting.
- `git diff --check`: passed.
