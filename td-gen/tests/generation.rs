use std::assert_matches;
use std::path::Path;
use std::{fs, io};

#[test]
fn simple() -> io::Result<()> {
  let tl_input = include_str!("fixtures/simple.tl");
  let rs_input = include_str!("fixtures/simple.rs");

  let tl_ast = td_parser::parse(tl_input).map_err(io::Error::other)?;
  let rs_output = td_gen::generate(&tl_ast).to_string();

  assert_eq!(rs_input, rs_output);

  Ok(())
}

#[test]
fn full() -> io::Result<()> {
  let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let tl_path = dir.join("../td/td_api.tl");
  let rs_path = dir.join("../td/td_api.rs");

  let tl_input = fs::read_to_string(tl_path)?;
  let tl_ast = td_parser::parse(&tl_input).map_err(|e| io::Error::other(e.to_string()))?;
  let rs_output = td_gen::generate(&tl_ast).to_string();

  fs::write(&rs_path, &rs_output)?;

  let [structs, enums, impls] = count_items(&rs_output).map_err(io::Error::other)?;
  assert_matches!(structs, 2606..);
  assert_matches!(enums, 737..);
  assert_matches!(impls, 1010..);

  Ok(())
}

fn count_items(rs: &str) -> syn::Result<[usize; 3]> {
  let [mut structs, mut enums, mut impls] = Default::default();
  let parsed = syn::parse_file(rs)?;
  let items = parsed.items.iter().flat_map(|i| match i {
    syn::Item::Mod(m) if let Some((_, items)) = &m.content => &**items,
    _ => &[],
  });

  for item in items {
    let count = match item {
      syn::Item::Struct(_) => &mut structs,
      syn::Item::Enum(_) => &mut enums,
      syn::Item::Impl(_) => &mut impls,
      _ => continue,
    };

    *count += 1
  }

  Ok([structs, enums, impls])
}
