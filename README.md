# Muzak

Muzak is a modern music player written in Rust using GPUI, designed to be
performant and lightweight while maximizing extensibility and maintaining a high
design standard.

# Features
- Fully native application with no web component
- FLAC, MP3, OGG and WAV playback
- Linux, macOS and (sort of) Windows support
- SQLite-backed library
- Theming with hot reload

## Planned Features
- Scrobbling (last.fm) support
- WASM Extension support:
  - Codecs
  - Scrobble services
  - Metadata services
- Playlists
- Advanced search
- AAC and Opus support

# Building
Muzak isn't ready for general use yet, but if you want to try it early:

```sh
# install relevant devel packages for xcb-common, x11, wayland, and pulseaudio if on Linux
git clone https://github.com/143mailliw/muzak
cd muzak
# debug mode will result in noticable slowdown
cargo run --release
```

# Contributing
If you make a pull request, try not to introduce any warnings (other than unused
enums/fields, which is fine if you're working on an API that could be used by
future extention support), and ensure your code was formatted with `rustfmt`
before submitting.

Please ensure any commits build on Linux, at the very least, and preferably macOS
too. The application is developed with running on Windows in mind, but it's
unsupported due to GPUI limitations.
