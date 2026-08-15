// Persists for the duration of the program
pub static TEMP_DIR: std::sync::LazyLock<tempfile::TempDir> = std::sync::LazyLock::new(|| {
    tempfile::tempdir().expect("To succeed creating a temporary directory")
});
