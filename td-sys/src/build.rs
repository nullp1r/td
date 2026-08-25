use std::env;
use std::path::Path;

/// Configures native link search paths, dynamic library linking, and rpaths
/// for `TDLib`.
///
/// This entry point is invoked by both `td-sys/build.rs` and dependent crate
/// build scripts. It performs the following configuration:
///
/// - **Search Path**: Emits `cargo:rustc-link-search=native` pointing to the
///   workspace `td/` directory.
/// - **Dynamic Linking**: Instructs `cargo` to link against `tdjson` when
///   building `td-sys`.
/// - **Linux**: Bakes `$ORIGIN` (deployment directory) and `{dir}` (local
///   build-time directory) into the ELF binary's `DT_RUNPATH`.
/// - **macOS**: Bakes `@executable_path`, `@loader_path`, and `{dir}` into
///   Mach-O `LC_RPATH` load commands.
pub fn link() {
  println!("cargo:rerun-if-changed=build.rs");

  let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../td");
  let dir = dir.canonicalize().unwrap_or(dir);
  let dir = dir.display();

  println!("cargo:rustc-link-search=native={dir}");
  if let Ok("td-sys") = env::var("CARGO_PKG_NAME").as_deref() {
    println!("cargo:rustc-link-lib=dylib=tdjson");
  }

  match env::var("CARGO_CFG_TARGET_OS").as_deref() {
    Ok("linux") => {
      println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
      println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
    Ok("macos") => {
      println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
      println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
      println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
    _ => {}
  }
}
