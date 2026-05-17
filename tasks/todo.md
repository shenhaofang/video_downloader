# Task 1: Scaffold Tauri App Skeleton

- [x] Check worktree status and confirm target scaffold files do not already exist.
- [x] Verify baseline failure for missing scaffold with `npm run build` and `cargo check`.
- [x] Create package metadata and frontend entry files.
- [x] Create Rust Tauri crate files.
- [x] Create Tauri config and default capability.
- [x] Add minimal standard support files if verification requires them.
- [x] Run `npm install`.
- [x] Run `npm run build`.
- [ ] Run `cargo check` in `src-tauri`.
- [x] Review diff and commit with `feat: scaffold tauri desktop app`.

## Review

- `npm install` completed successfully and generated `package-lock.json`.
- `npm run build` completed successfully after adding the minimal Vite/TypeScript support files.
- `cargo check` could not run in this environment because `cargo`, `rustc`, and `rustup` are not available on PATH or in the standard user Cargo directory.
- Plan correction: added `tsconfig.json`, `frontend/src/vite-env.d.ts`, `src-tauri/build.rs`, and artifact ignore entries in `.gitignore` as minimal scaffold support.
