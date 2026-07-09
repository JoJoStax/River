# River
A blazing-fast, cross-platform media hub built in Rust—designed for user sovereignty, freedom of expression, and genuine community-driven development.

## Building & Running

### Desktop (Linux/macOS/Windows)
To run River locally on your desktop:
```bash
cargo run -p river-render
```

### Android APK (`cargo-apk`)
Ensure you have `cargo-apk` and the Android NDK installed (`cargo install cargo-apk`). Then run:
```bash
# Build debug APK
cargo apk build -p river-render

# Build release APK
cargo apk build --release -p river-render
```
The built APK will be located inside `target/debug/apk/river-render.apk` or `target/release/apk/river-render.apk`.
