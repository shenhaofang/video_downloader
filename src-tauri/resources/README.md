# Installer Resources

This directory contains small installer resources that are bundled into the
Windows NSIS installer.

`ffmpeg` and `ffprobe` are distributed as a pinned LGPL-compatible FFmpeg
archive published as the GitHub Release asset `ffmpeg-win64-lgpl.zip`. The
Windows NSIS installer runs `install-media-tools.ps1` after installation. The
script skips work when both tools already exist, otherwise it downloads the
pinned archive, verifies its SHA256 checksum, and extracts the tools to:

`dependencies\ffmpeg\bin\`

The `dependencies` directory is created directly under the program installation
root. The app checks that installer-managed location when the user has not
configured custom media tool paths in Settings.
