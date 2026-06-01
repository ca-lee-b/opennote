fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=src/transcription/macos_permissions_bridge.m");

        cc::Build::new()
            .file("src/transcription/macos_permissions_bridge.m")
            .flag("-fobjc-arc")
            .compile("opennote_macos_permissions_bridge");

        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
    }

    tauri_build::build()
}
