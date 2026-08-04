//! Copy the `assets/` tree into the build output directory so Bevy's
//! `AssetServer` finds it next to the executable at runtime.
//!
//! ## Why a build script
//! Bevy resolves asset paths relative to the *executable* (or `BEVY_ASSET_ROOT`
//! env var), not the CWD. With `cargo run` / `cargo build`, the exe lands in
//! `target/<profile>/voxelforge.exe`, so it looks for `target/<profile>/assets/`.
//! The canonical `assets/` folder lives at the workspace root — the build script
//! bridges that gap automatically on every build.
//!
//! ## Coverage
//! `OUT_DIR` is always inside the active target directory, so this works with
//! every `--target-dir` the team uses (`target/`, `target-quest/`,
//! `target-poppy/`, `target-flamingo/`, `target-anim/`) and across all profiles
//! (debug, release, perf).

use std::path::Path;
use std::{fs, io};

fn main() {
    // Re-run the build script whenever any source file in this package
    // changes — not just when the asset tree changes. Without this, a
    // code-only rebuild (e.g. after a fresh checkout or after someone wipes
    // the target dir) would recompile the binary but skip the asset copy,
    // leaving the exe deaf. Watching src/ + Cargo.toml closes that gap.
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");
    // Re-run when the shared asset tree at the workspace root changes.
    println!("cargo:rerun-if-changed=../assets/");

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("assets");

    let out = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    // OUT_DIR = <target>/<profile>/build/<crate>-<hash>/out
    //   → .ancestors().nth(3) = <target>/<profile>
    let target_profile = Path::new(&out)
        .ancestors()
        .nth(3)
        .expect("OUT_DIR has fewer than 3 ancestors");
    let dst = target_profile.join("assets");

    if let Err(e) = copy_assets(&src, &dst) {
        // `cargo:warning=` prints to stderr as a build warning so the team
        // sees it — an asset copy failure should never be silent.
        println!(
            "cargo:warning=asset copy failed: {} → {} : {e}",
            src.display(),
            dst.display()
        );
    }
}

/// Recursive copy: overwrites everything so a stale asset never outlives the
/// source. Fast enough for the ~25 small files shipped in v1 (< 2 MB total).
fn copy_assets(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("source assets dir not found: {}", src.display()),
        ));
    }

    // Clean destination so deleted source files don't linger across builds.
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;

    copy_dir(src, dst)
}

fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst = dst.join(entry.file_name());
        if ty.is_dir() {
            fs::create_dir_all(&dst)?;
            copy_dir(&entry.path(), &dst)?;
        } else {
            fs::copy(entry.path(), &dst)?;
        }
    }
    Ok(())
}
