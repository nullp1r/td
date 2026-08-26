//! Links `td-sys` to the native `TDLib` library.

#[path = "src/build.rs"]
mod build;

fn main() {
  build::link();
}
