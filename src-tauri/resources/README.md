# Installer Resources

This directory contains installer resources that are bundled into Windows NSIS
installers.

`ffmpeg` and `ffprobe` are distributed as a pinned LGPL-compatible FFmpeg
archive. The full setup installer bundles
`resources/vendor/ffmpeg/ffmpeg-win64-lgpl.zip`, so first install does not need
to download required media tools. The slim app-update installer bundles only
`install-media-tools.ps1`; when dependencies are missing, the script downloads
the pinned GitHub Release asset, verifies its SHA256 checksum, and extracts the
tools to:

`dependencies\ffmpeg\bin\`

The `dependencies` directory is created directly under the program installation
root. The app checks that installer-managed location when the user has not
configured custom media tool paths in Settings.
