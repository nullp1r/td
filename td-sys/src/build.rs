use std::env;
use std::path::Path;

/// Configures link search path, dynamic library linking, and rpaths for `TDLib`.
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
