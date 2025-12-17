//! uniffi-bindgen helper binary
//!
//! Usage: cargo run --bin uniffi-bindgen --features bindgen -- <library_path> <out_dir>
//! Example: cargo run --bin uniffi-bindgen --features bindgen -- target/debug/libactr_kotlin.dylib kotlin-bindings

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <library_path> <out_dir>", args[0]);
        eprintln!(
            "Example: {} target/debug/libactr_kotlin.dylib kotlin-bindings",
            args[0]
        );
        std::process::exit(1);
    }

    let library_path = camino::Utf8PathBuf::from(&args[1]);
    let out_dir = camino::Utf8PathBuf::from(&args[2]);

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&out_dir)?;

    println!(
        "Generating Kotlin bindings from {} to {}",
        library_path, out_dir
    );

    uniffi_bindgen::library_mode::generate_bindings(
        &library_path,
        None, // crate_name
        &uniffi_bindgen::bindings::KotlinBindingGenerator,
        &uniffi_bindgen::EmptyCrateConfigSupplier,
        None, // config_file_override
        &out_dir,
        false, // try_format_code
    )?;

    println!("Kotlin bindings generated successfully!");
    Ok(())
}
