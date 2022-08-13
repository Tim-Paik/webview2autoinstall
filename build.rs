#![cfg(windows)]
#[cfg(feature = "bin")]
use embed_manifest::{embed_manifest, new_manifest};

#[cfg(feature = "bin")]
fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest(new_manifest("WebView2AutosInstall"))
            .expect("unable to embed manifest file");
    }
    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(not(feature = "bin"))]
fn main() {}
