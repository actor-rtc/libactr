fn main() {
    // Generate UniFFI bindings
    uniffi::generate_scaffolding("src/actr_kotlin.udl").unwrap();
}
