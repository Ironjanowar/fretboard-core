//! `uniffi-bindgen` for the Fretboard mobile bindings.
//!
//! The binary only delegates to the UniFFI CLI pinned in the workspace. Keeping
//! the generator inside the workspace guarantees that generated Kotlin matches
//! the runtime linked into `fretboard-mobile-ffi`.

fn main() {
    uniffi::uniffi_bindgen_main()
}
