use std::env;
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
    }
    tauri_build::build()
}