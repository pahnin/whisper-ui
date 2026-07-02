use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os != "macos" {
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let info_plist_path = PathBuf::from(manifest_dir).join("Info.plist");

    if !info_plist_path.exists() {
        eprintln!(
            "[whisper-app build] Info.plist not found at {}. macOS features will be unavailable.",
            info_plist_path.display()
        );
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dst_plist = out_dir.join("Info.plist");

    // Copy Info.plist to OUT_DIR so we can reference it
    fs::copy(&info_plist_path, &dst_plist).expect("Failed to copy Info.plist");

    // Get the absolute path for the linker
    let abs_plist = std::fs::canonicalize(&dst_plist).unwrap_or(dst_plist);

    // Pass the plist path to the macOS linker
    // The linker flag tells macOS to embed the Info.plist in the binary
    println!(
        "cargo:rustc-link-arg=-Wl,-info_plist_path,{}",
        abs_plist.display()
    );

    // Also set deployment target
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.14");
}
