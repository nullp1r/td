use std::fmt;

const KEYWORDS: [&str; 52] = [
  "Self", "abstract", "as", "async", "await", "become", "box", "break", //.
  "const", "continue", "crate", "do", "dyn", "else", "enum", "extern", //.
  "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", //.
  "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", //.
  "return", "self", "static", "struct", "super", "trait", "true", "try", "type", //.
  "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

pub fn escape_rust_keyword(s: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    if KEYWORDS.contains(&s) {
      f.write_str("r#")?;
    }
    f.write_str(s)
  })
}

pub fn tl_type_to_rust(name: &str) -> Option<&'static str> {
  match name {
    "Int32" | "int32" => Some("i32"),
    "Int53" | "int53" => Some("i64"),
    "Int64" | "int64" => Some("i64"),
    "Double" | "double" => Some("f64"),
    "String" | "string" => Some("String"),
    "Vector" | "vector" => Some("Vec<_>"),
    "Bytes" | "bytes" => Some("Vec<u8>"),
    "Bool" | "bool" => Some("bool"),
    _ => None,
  }
}
