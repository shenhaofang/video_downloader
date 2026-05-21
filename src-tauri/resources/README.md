# Installer Resources

This directory contains installer resources that are bundled into the Windows
NSIS installer.

`ffmpeg` and `ffprobe` are distributed as a pinned LGPL-compatible FFmpeg
archive under `resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip`. The Windows
NSIS installer bundles that archive into the setup executable, verifies its
SHA256 checksum during installation, and extracts the tools to:

`dependencies\ffmpeg\bin\`

The `dependencies` directory is created directly under the program installation
root. The app checks that installer-managed location when the user has not
configured custom media tool paths in Settings.
