fn main() {
    // Unit-test/library builds do not enable the optional Tauri dependency, so
    // tauri-build cannot read DEP_TAURI_DEV. Desktop bundles still run the full build helper.
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        tauri_build::build()
    }
}
