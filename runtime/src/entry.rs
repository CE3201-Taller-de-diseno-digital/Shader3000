#![crate_type = "bin"]

#[cfg(not(target_family = "unix"))]
error!("Unsupported target for entry point");

fn main() {
    extern "Rust" {
        fn handover();
    }

    unsafe {
        handover();
    }
}
