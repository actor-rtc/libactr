# libactr - Rust FFI Layer for ACTR Kotlin Bindings

This is the Rust FFI layer for the actr-kotlin project, providing Kotlin bindings for the Actor-RTC (actr) framework using Mozilla UniFFI.

## Overview

This crate provides the native implementation that gets compiled to a dynamic library (`libactr_kotlin.dylib` on macOS, `libactr_kotlin.so` on Linux, etc.) and exposes a C-compatible API that UniFFI uses to generate type-safe Kotlin bindings.

## Architecture

- **UniFFI Interface**: `actr_kotlin.udl` - Defines the interface between Rust and Kotlin
- **Core Types**: `types.rs` - Core ACTR types (ActrId, ActrType, ActrConfig, etc.)
- **Runtime**: `runtime.rs` - ACTR client wrapper and system management
- **Workload**: `workload.rs` - Workload callback interfaces
- **Error Handling**: `error.rs` - Error types and conversions
- **Library**: `lib.rs` - UniFFI scaffolding and exports

## Building

### Prerequisites

- Rust 1.88+ with `rustup`
- The actr workspace dependencies (protocol, framework, runtime, config)

### Build Commands

```bash
# Build the release library
cargo build --release

# The output will be target/release/libactr_kotlin.dylib (macOS)
# or target/release/libactr_kotlin.so (Linux)
```

### Generate Kotlin Bindings

After building the library, generate the Kotlin bindings using UniFFI:

```bash
# Generate Kotlin bindings from the compiled library
cargo run --bin uniffi-bindgen --features bindgen -- target/release/libactr_kotlin.dylib kotlin/src/main/kotlin 2>&1
```

This command will:
1. Run the `uniffi-bindgen` binary with the `bindgen` feature enabled
2. Use the compiled `libactr_kotlin.dylib` library
3. Generate Kotlin bindings in `kotlin/src/main/kotlin/`
4. Output any warnings or errors to stderr

### Expected Output

Successful binding generation will show:
```
Generating Kotlin bindings from target/release/libactr_kotlin.dylib to kotlin/src/main/kotlin
Kotlin bindings generated successfully!
```

## Development

### Code Organization

- **UDL Interface** (`actr_kotlin.udl`): Defines the FFI interface that UniFFI uses to generate bindings
- **Rust Implementation**: Core logic in `src/` files
- **Generated Kotlin**: Auto-generated bindings in `../kotlin/src/main/kotlin/`

### Adding New Types/Functions

1. Update the `.udl` file to define new interfaces
2. Implement the Rust side in the appropriate `src/*.rs` file
3. Mark functions with `#[uniffi::export]` for exposure to Kotlin
4. Regenerate bindings after changes

### Testing

```bash
# Run Rust tests
cargo test

# Run with bindgen feature
cargo test --features bindgen
```

## Dependencies

This crate depends on the actr workspace crates:
- `actr-protocol`: Core protocol definitions
- `actr-framework`: Actor framework
- `actr-runtime`: Runtime implementation
- `actr-config`: Configuration management

## Features

- `bindgen`: Enables the UniFFI binding generator binary
- `default`: Empty (no default features)

## Troubleshooting

### Common Warnings

- `associated function 'new' is never used`: This is expected for some internal constructors that are only used through FFI

### Build Issues

- Ensure all workspace dependencies are available
- Check that `protoc` is installed for protobuf compilation
- Verify Rust version meets minimum requirements (1.88+)

### Binding Generation Issues

- Ensure the library is built with `--release` before generating bindings
- Check that the output path `kotlin/src/main/kotlin` exists
- Verify UniFFI version compatibility

## Integration

This Rust library is consumed by:
- **Kotlin Library**: `../kotlin/` - Contains the generated bindings and Kotlin wrappers
- **Android Demo**: `../android-demo/` - Android application using the bindings

The generated Kotlin bindings provide type-safe access to ACTR functionality from Android applications.</content>
<parameter name="filePath">/Users/mafeng/Desktop/dev/actr-kotlin/rust/README.md