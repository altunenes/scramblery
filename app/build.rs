use std::env;

fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint UI failed to compile");

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

    // Copy the WebGPU Dawn dylib to a known location for bundling.
    // ort's copy-dylibs places it in the target profile dir; the packager config
    // references webgpu-dylibs/ so it can be found regardless of the target triple.
    if env::var_os("CARGO_FEATURE_WEBGPU").is_some() {
        let out_dir = std::path::PathBuf::from(env::var("OUT_DIR").unwrap());
        // OUT_DIR is something like target/{triple}/release/build/{pkg}/out
        // Walk up to find the profile dir (where ort places the dylib)
        if let Some(profile_dir) = out_dir.ancestors().nth(3) {
            let dylib_name = match target.as_deref() {
                Ok("macos") => "libwebgpu_dawn.dylib",
                Ok("windows") => "webgpu_dawn.dll",
                _ => "libwebgpu_dawn.so",
            };
            let src = profile_dir.join(dylib_name);
            if src.exists() {
                let manifest_dir = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
                let dest_dir = manifest_dir.join("webgpu-dylibs");
                std::fs::create_dir_all(&dest_dir).ok();
                let dest = dest_dir.join(dylib_name);
                std::fs::copy(&src, &dest).ok();
                println!("cargo:warning=Copied {} to {}", src.display(), dest.display());
            }
        }
    }
}
