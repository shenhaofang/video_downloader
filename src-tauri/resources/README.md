# Installer Resources

This directory contains small installer resources that are bundled into the
Windows NSIS installer.

`ffmpeg` and `ffprobe` binaries are not committed and are not Tauri sidecars.
The NSIS installer downloads a pinned LGPL FFmpeg archive during installation,
verifies its SHA256 checksum, and extracts the tools to:

`resources\media-tools\ffmpeg\bin\`

The app checks that installer-managed location when the user has not configured
custom media tool paths in Settings.
