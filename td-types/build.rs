use std::path::Path;
use std::{env, fs, io};

fn main() -> io::Result<()> {
  let manifest_dir = env!("CARGO_MANIFEST_DIR");
  let out_dir = env::var_os("OUT_DIR").ok_or(io::Error::other("OUT_DIR not set"))?;

  let tl_path = Path::new(manifest_dir).join("../td/td_api.tl");
  let rs_path = Path::new(&out_dir).join("generated.rs");

  println!("cargo:rerun-if-changed={}", tl_path.display());

  let tl_input = fs::read_to_string(tl_path)?;
  let tl_ast = td_parser::parse(&tl_input).map_err(|e| io::Error::other(e.to_string()))?;
  let rs_output = td_gen::generate(&tl_ast).to_string();

  fs::write(rs_path, rs_output)?;

  Ok(())
}
