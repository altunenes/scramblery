**Prerequisites:**

*   **Rust:**  [Install Rust](https://www.rust-lang.org/tools/install) (stable version recommended).
*   **Node.js and npm:** [Install Node.js and npm](https://nodejs.org/).
*   **Tauri CLI:**  Install Tauri CLI globally: `npm install -g @tauri-apps/cli`

*   **GStreamer (for Video Processing):**
    * Install both the runtime and development packages: https://gstreamer.freedesktop.org/download/
    *   **macOS:** GStreamer framework is expected to be installed (see `ui/src-tauri/build.rs` for hints). You might need to install it via Homebrew: `brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-ugly gst-plugins-bad`
    *   **Linux/Windows:**  Installation of GStreamer development packages is required. 

**Build Instructions:**

1.  **Navigate to the UI directory:**
    ```bash
    cd ui
    ```

2.  **Install npm dependencies:**
    ```bash
    npm install
    ```

3.  **Run the Tauri development command:**
    ```bash
    npm run tauri dev
    ```