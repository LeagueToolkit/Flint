fn main() {
    // Only run Tauri build when building with GUI support
    #[cfg(feature = "tauri-ui")]
    tauri_build::build();
}

