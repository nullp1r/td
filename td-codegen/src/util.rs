use std::fmt;

use td_parser::TypeExpr;

pub fn is_bytes(expr: &TypeExpr) -> bool {
  matches!(expr, TypeExpr::Bare("bytes"))
}

pub fn is_int64(expr: &TypeExpr) -> bool {
  matches!(expr, TypeExpr::Bare("int64"))
}

pub fn is_int64_vec(expr: &TypeExpr) -> bool {
  matches!(expr, TypeExpr::Vector(inner) if is_int64(inner))
}

pub fn to_native(name: &str) -> Option<&'static str> {
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

pub fn escaped_keyword(s: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    if RUST_KEYWORDS.contains(&s) {
      f.write_str("r#")?;
    }
    f.write_str(s)
  })
}

const RUST_KEYWORDS: [&str; 52] = [
  "Self", "abstract", "as", "async", "await", "become", "box", "break", //.
  "const", "continue", "crate", "do", "dyn", "else", "enum", "extern", //.
  "false", "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", //.
  "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", //.
  "return", "self", "static", "struct", "super", "trait", "true", "try", "type", //.
  "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];
