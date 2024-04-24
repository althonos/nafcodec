extern crate built;

fn main() {
    let dst = std::env::var("OUT_DIR")
        .map(std::path::PathBuf::from)
        .expect("OUT_DIR not set")
        .join("built.rs");
    let project_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .expect("CARGO_MANIFEST_DIR");

    eprintln!(
        "{:?}",
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")
    );
    built::write_built_file_with_opts(Some(&project_dir.parent().unwrap()), &dst)
        .expect("Failed to acquire build-time information");
}
