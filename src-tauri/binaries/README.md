# Bundled Media Binaries

This directory is reserved for Tauri sidecar binaries used by media tooling.

The app expects these sidecar base names:

- `ffmpeg`
- `ffprobe`

Tauri sidecars are named with the target triple suffix for each build target.
For Windows x86_64 MSVC, the expected files are:

- `ffmpeg-x86_64-pc-windows-msvc.exe`
- `ffprobe-x86_64-pc-windows-msvc.exe`

Use `rustc --print host-tuple` to inspect the local host tuple. Before any
public distribution, verify that the bundled FFmpeg build and enabled codecs
match the intended license profile.
