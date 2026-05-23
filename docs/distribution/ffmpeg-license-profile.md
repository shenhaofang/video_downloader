# FFmpeg License Profile

This document records the distribution profile for installer-bundled `ffmpeg` and `ffprobe` binaries.
It is an engineering release gate, not legal advice.

## Default First-Release Profile

- Use an LGPL-compatible FFmpeg build for any installer-managed public distribution.
- The build must not use `--enable-gpl`.
- The build must not use `--enable-nonfree`.
- If the project later needs GPL-enabled codecs or libraries, that change must be reviewed as a separate release decision before the binary is installed by the app installer.
- Nonfree FFmpeg builds are not distributable in this app.
- The Windows full installer bundles `src-tauri/resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip`, copied from BtbN FFmpeg-Builds `autobuild-2026-05-18-18-09` asset `ffmpeg-n7.1.4-5-ged860ef7d9-win64-lgpl-shared-7.1.zip`, verified with SHA256 `d3c0d41c26b64bb42abbf9051a9494bc67185b6d9fa57798f20efb0e0213caf7`.
- The Windows slim app-update installer does not bundle that archive. If either required tool is missing under the install-root `dependencies\ffmpeg` directory, it downloads the same GitHub Release asset and verifies the same SHA256 before installation.

## Required Binary Record

For every installer-managed `ffmpeg` / `ffprobe` binary, keep this record with the release artifacts:

- FFmpeg version and source revision.
- Binary provider or build pipeline.
- Target OS and architecture.
- Full `./configure` line.
- Confirmation that `--enable-gpl` and `--enable-nonfree` are absent.
- Included external libraries and their licenses.
- Unmodified FFmpeg license files, including the LGPL text that applies to the build.
- Source offer or source archive location for the exact binary build.
- Any local patches as a separate diff.

## Packaging Rule

The app may support user-configured local tool paths during development and internal testing.
Before a public installer bundles, downloads, or installs FFmpeg binaries, the release must attach a completed binary record for each target platform and verify that the bundled archive path, release asset URL, checksum, and extracted tool paths match the recorded build.

## References

- FFmpeg legal notes: https://ffmpeg.org/legal.html
- FFmpeg license notes: https://www.ffmpeg.org/doxygen/4.3/md_LICENSE.html
