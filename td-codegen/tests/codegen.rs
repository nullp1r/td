use std::assert_matches;
use std::path::Path;
use std::{fs, io};

#[test]
fn fixtures() -> io::Result<()> {
  let tl_input = include_str!("fixtures/simple.tl");
  let rs_input = include_str!("fixtures/simple.rs");

  let tl_ast = td_parser::parse(tl_input).map_err(io::Error::other)?;
  let rs_output = td_codegen::generate(&tl_ast).to_string();

  assert_eq!(rs_input, rs_output);

  Ok(())
}

#[test]
fn upstream() -> io::Result<()> {
  let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let tl_path = dir.join("../td/td_api.tl");
  let rs_path = dir.join("../td/td_api.rs");

  let tl_input = fs::read_to_string(tl_path)?;
  let tl_ast = td_parser::parse(&tl_input).map_err(|e| io::Error::other(e.to_string()))?;
  let rs_output = td_codegen::generate(&tl_ast).to_string();

  fs::write(&rs_path, &rs_output)?;

  let [mut structs, mut enums, mut impls] = Default::default();
  let parsed = syn::parse_file(&rs_output).map_err(io::Error::other)?;
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

    *count += 1;
  }

  assert_matches!(structs, 2606..);
  assert_matches!(enums, 737..);
  assert_matches!(impls, 1010..);

  Ok(())
}

#[test]
#[ignore = "benchmark"]
#[expect(clippy::assertions_on_constants, reason = "release only")]
fn throughput() -> io::Result<()> {
  use std::fmt::Write;
  use std::hint::black_box;
  use std::time::Instant;

  // cargo test --release -p td-codegen throughput -- --ignored --nocapture
  assert!(!cfg!(debug_assertions), "must be run with `--release`");

  let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let tl_path = dir.join("../td/td_api.tl");
  let tl_input = fs::read_to_string(tl_path)?;
  let tl_ast = td_parser::parse(&tl_input).map_err(|e| io::Error::other(e.to_string()))?;

  let iters = 100;

  let mut buf = String::new();

  let t0 = Instant::now();
  for _ in 0..iters {
    let ast = td_parser::parse(black_box(&tl_input)).map_err(|e| io::Error::other(e.to_string()))?;
    black_box(ast);
  }
  let t1 = Instant::now();
  for _ in 0..iters {
    buf.clear();
    let _ = write!(&mut buf, "{}", td_codegen::generate(black_box(&tl_ast)));
    black_box(&buf);
  }
  let t2 = Instant::now();

  let [parsing_lines, codegen_lines] = [&tl_input, &buf].map(|s| iters * s.lines().count());
  let parsing_lines_s = parsing_lines as f64 / t1.duration_since(t0).as_secs_f64();
  let codegen_lines_s = codegen_lines as f64 / t2.duration_since(t1).as_secs_f64();

  println!("parsing: {parsing_lines_s:.0} lines/s");
  println!("codegen: {codegen_lines_s:.0} lines/s");

  Ok(())
}
