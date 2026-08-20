use std::env;
use std::path::Path;

fn main() {
  println!("cargo:rerun-if-changed=build.rs");

  let td_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../td");
  let dir = td_dir.canonicalize().unwrap_or(td_dir);
  let dir_str = dir.display();

  println!("cargo:rustc-link-search=native={dir_str}");
  println!("cargo:rustc-link-lib=dylib=tdjson");

  match env::var("CARGO_CFG_TARGET_OS").as_deref() {
    Ok("linux") => {
      println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
      println!("cargo:rustc-link-arg=-Wl,-rpath,{dir_str}");
    }
    Ok("macos") => {
      println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
      println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
      println!("cargo:rustc-link-arg=-Wl,-rpath,{dir_str}");
    }
    _ => {}
  }
}
