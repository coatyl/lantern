use std::process::Command;

fn main() {
    // Capture the rustc version at compile time so the About pane can show it
    // without shelling out at runtime.  Falls back to "unknown" if the
    // toolchain doesn't produce a parseable string; the About pane treats
    // this as informational.
    let rustc_version = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .arg("-V")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=LANTERN_RUSTC_VERSION={rustc_version}");

    // Forward the LANTERN_SIGNED env var (set by the CI sign-windows job
    // after `signtool` returns 0) so `option_env!("LANTERN_SIGNED")` in
    // commands.rs can read it.  Unset → empty → reported as `signed=false`.
    println!("cargo:rerun-if-env-changed=LANTERN_SIGNED");
    if let Ok(signed) = std::env::var("LANTERN_SIGNED") {
        println!("cargo:rustc-env=LANTERN_SIGNED={signed}");
    }

    // tauri-build embeds its application manifest into the app binary only,
    // so `cargo test` executables that link the Tauri runtime start without
    // one and Windows refuses to load them (STATUS_ENTRYPOINT_NOT_FOUND on
    // comctl32).  Embed our own manifest through the linker instead, which
    // applies to every target.
    let mut windows = tauri_build::WindowsAttributes::new();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("tauri-build failed");
}
