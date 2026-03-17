use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("CARGO_CFG_TARGET_OS");
    if target == Ok("macos".to_string()) {
        env::set_var(
            "PKG_CONFIG_PATH",
            "/Library/Frameworks/GStreamer.framework/Versions/Current/lib/pkgconfig",
        );
        let lib = "/Library/Frameworks/GStreamer.framework/Versions/Current/lib";
        env::set_var("GST_PLUGIN_PATH", lib);
        env::set_var("DYLD_FALLBACK_LIBRARY_PATH", lib);
        println!("cargo:rustc-link-search=framework=/Library/Frameworks");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Library/Frameworks/GStreamer.framework/Versions/Current/lib");
        // Allow loading dylibs bundled in the .app (Contents/Frameworks/)
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }

    // Copy WebGPU Dawn dylib to a known location for Tauri bundling (macOS).
    // ort's copy-dylibs places it in the target profile dir; tauri.webgpu.conf.json
    // references webgpu-dylibs/ so it can be found regardless of the target triple.
    #[cfg(feature = "webgpu")]
    {
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        // OUT_DIR is something like target/{triple}/release/build/{pkg}/out
        // Walk up to find the profile dir (where ort places the dylib)
        if let Some(profile_dir) = out_dir.ancestors().nth(3) {
            let dylib_name = if cfg!(target_os = "macos") {
                "libwebgpu_dawn.dylib"
            } else if cfg!(target_os = "windows") {
                "webgpu_dawn.dll"
            } else {
                "libwebgpu_dawn.so"
            };
            let src = profile_dir.join(dylib_name);
            if src.exists() {
                let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
                let dest_dir = manifest_dir.join("webgpu-dylibs");
                std::fs::create_dir_all(&dest_dir).ok();
                let dest = dest_dir.join(dylib_name);
                std::fs::copy(&src, &dest).ok();
                println!("cargo:warning=Copied {} to {}", src.display(), dest.display());
            }
        }
    }

    tauri_build::build()
}