# HomeTube

HomeTube is a Linux-first Tauri desktop library with a native YouTube-to-MP4
download queue. It combines automatic local indexing, playlist downloads,
channel-based rows, favorites, viewing progress, and embedded playback in one
local-only application.

## Requirements

- Rust 1.77.2 or newer
- Node.js 20 or newer
- `yt-dlp`, `ffmpeg`, and `ffprobe` on `PATH`
- WebKitGTK and the standard Tauri Linux build dependencies

On Arch Linux, the application dependencies are typically available through
`base-devel`, `webkit2gtk-4.1`, `libappindicator-gtk3`, `librsvg`, `ffmpeg`, and
your preferred yt-dlp package.

## Development

```bash
npm install
npm run tauri dev
```

Discover accepts public YouTube video and playlist links. Downloads run one at
a time, resume after an application restart, and are normalized to compatible
MP4 files before appearing in the library.

## Verification

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Build the Linux AppImage with `npm run build:appimage`. This sets linuxdeploy's
`NO_STRIP=1` compatibility flag because modern Arch libraries use RELR sections
that the bundler's older embedded `strip` does not understand. The build includes the
GStreamer media framework required by WebKitGTK. yt-dlp and FFmpeg remain system
dependencies; downloaded codecs are normalized automatically when necessary.

For manually assembled AppDirs, run `./scripts/fix-appimage-gstreamer.sh` before
compression. It preserves host plugin discovery and stages the installed
GStreamer plugins in the directory expected by linuxdeploy.

## Privacy and scope

All indexing data, favorites, progress, artwork, and compatible playback copies
stay in HomeTube's local application-data directory. HomeTube does not provide
DRM bypassing, hosted streaming, user accounts, or cloud synchronization.
