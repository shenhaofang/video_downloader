# FFmpeg License Profile

This document records the distribution profile for bundled `ffmpeg` and `ffprobe` binaries.
It is an engineering release gate, not legal advice.

## Default First-Release Profile

- Use an LGPL-compatible FFmpeg build for any bundled public distribution.
- The build must not use `--enable-gpl`.
- The build must not use `--enable-nonfree`.
- If the project later needs GPL-enabled codecs or libraries, that change must be reviewed as a separate release decision before the binary is bundled.
- Nonfree FFmpeg builds are not distributable in this app.

## Required Binary Record

For every bundled `ffmpeg` / `ffprobe` binary, keep this record with the release artifacts:

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
Before a public installer bundles FFmpeg binaries, the release must attach a completed binary record for each target platform and verify that the Tauri sidecar names match the target-triple binaries.

## References

- FFmpeg legal notes: https://ffmpeg.org/legal.html
- FFmpeg license notes: https://www.ffmpeg.org/doxygen/4.3/md_LICENSE.html
