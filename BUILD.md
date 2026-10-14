**Prerequisites:**

*   **Rust:** [Install Rust](https://www.rust-lang.org/tools/install) (stable version recommended).
*   **GStreamer (for Video Processing):**
    *   Install both the runtime and development packages: https://gstreamer.freedesktop.org/download/
    *   **macOS:** the official GStreamer framework installer is expected (`/Library/Frameworks/GStreamer.framework`, see `app/build.rs`).
    *   **Linux:** `libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev` plus the `gstreamer1.0-plugins-*` packages. The UI also needs `libfontconfig1-dev libxkbcommon-dev`.
    *   **Windows:** install the MSVC runtime and development installers.

**Run:**

From the repository root:

```bash
cargo run --release
```

For optical flow on the GPU, add a feature: `cargo run --release --features webgpu` (also `cuda`, `directml`, `openvino`, `migraphx`).

**Command line:**

```bash
cargo run --release -p scramblery-cli -- --help
cargo run --release -p scramblery-cli -- config fourier > fourier.json   # edit, then:
cargo run --release -p scramblery-cli -- scramble input.png -o output.png -c fourier.json --seed 42
cargo run --release -p scramblery-cli -- scramble images/ -o scrambled/ -c fourier.json
cargo run --release -p scramblery-cli -- scramble images/ -o series/ -c fourier.json --exemplars 3 --levels 0,25,50,75,100 --match histogram
cargo run --release -p scramblery-cli -- analyze output.png --reference input.png
cargo run --release -p scramblery-cli -- normalize images/ -o normalized/ --method luminance
```

**Tests:**

```bash
cargo test --workspace
```

`engine/tests/properties.rs` checks the properties each method is documented to preserve. On macOS, set `DYLD_FALLBACK_LIBRARY_PATH=/Library/Frameworks/GStreamer.framework/Versions/Current/lib` so the test binaries find GStreamer.

**Project layout:**

*   `app/`: the desktop app, built with [Slint](https://slint.dev). `app/ui/` holds the interface, `app/src/` the Rust code.
*   `cli/`: the `scramblery-cli` command line tool.
*   `engine/`: scrambling, analysis, normalization, batch and video processing.
*   `face_detection/`, `optical_flow/`, `onnx/`: ONNX Runtime models.
*   `video-processor/`: GStreamer decoding and encoding.
*   `resources/models/`: ONNX model files.

**Package an installer:**

Installers are made with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager):

```bash
cargo install cargo-packager --locked
cargo build --release
cd app
cargo packager --release --formats app      # macOS .app (dmg also works)
cargo packager --release --formats nsis     # Windows installer
cargo packager --release --formats deb,appimage
```

Packages are written to `target/packages/`. The configuration is in `[package.metadata.packager]` in `app/Cargo.toml`.

### Extras

**Q: Where does the app look for the optical flow model?**
*A: Next to the executable (`models/`), in `../Resources/models` inside a macOS `.app`, in `../lib/scramblery/models` for Linux packages, then in the repository's `resources/models`. Set `SCRAMBLERY_MODELS_DIR` to override.*

**Q: For development, how can I embed GStreamer in the app?**
*A: On macOS, add the framework to the packager config in `app/Cargo.toml`:*

```toml
[package.metadata.packager.macos]
frameworks = ["/Library/Frameworks/GStreamer.framework"]
```
