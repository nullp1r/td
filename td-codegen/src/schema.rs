//! Canonical Type Language schema catalog, indexing, and wire mappings.

use std::fmt;

use td_parser::{Definition, DefinitionKind, TypeExpr};

use crate::layout::Layout;

/// Deterministic schema indexes and layout analysis used during generation.
pub struct Schema<'a> {
  /// Constructors sorted by `[result_type, constructor_name]`.
  ctors: Vec<&'a Definition<'a>>,
  /// Functions retained in schema order.
  fns: Vec<&'a Definition<'a>>,
  /// Sorted, deduplicated TL type names for binary search.
  type_names: Vec<&'a str>,
  /// Recursive layout cycle detector.
  layout: Layout<'a>,
}

impl<'a> Schema<'a> {
  /// Builds deterministic lookup tables for `ast`.
  pub fn new(ast: &'a [Definition<'a>]) -> Self {
    let (mut type_names, mut ctors, mut fns): (Vec<_>, Vec<_>, Vec<_>) = Default::default();

    for def in ast {
      let target = match def.kind {
        DefinitionKind::Type => &mut ctors,
        DefinitionKind::Function => &mut fns,
      };
      target.push(def);
      type_names.push(def.r#type);
    }

    ctors.sort_unstable_by_key(|def| [def.r#type, def.name]);

    type_names.sort_unstable();
    type_names.dedup();

    let layout = Layout::new(ast);
    Self { ctors, fns, type_names, layout }
  }

  /// Iterates non-primitive TL types and their contiguous constructor groups.
  pub fn types(&self) -> impl Iterator<Item = (&'a str, &[&'a Definition<'a>])> {
    self.ctors.chunk_by(|a, b| a.r#type == b.r#type).filter_map(|group| {
      let &[def, ..] = group else { return None };
      let None = primitive(def.r#type) else { return None };
      Some((def.r#type, group))
    })
  }

  /// Returns function definitions in their original schema order.
  pub fn fns(&self) -> &[&'a Definition<'a>] {
    &self.fns
  }

  /// Reports whether `name` is a known TL type.
  pub fn is_type(&self, name: &str) -> bool {
    self.type_names.binary_search(&name).is_ok()
  }

  /// Reports whether a reference from `parent` to `target` forms a layout cycle.
  pub fn is_recursive(&self, parent: &str, target: &str) -> bool {
    self.layout.is_recursive(parent, target)
  }
}

/// Maps a TL primitive name to its generated Rust type.
pub fn primitive(name: &str) -> Option<&'static str> {
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

/// Returns the Serde attribute module path if `expr` requires a custom adapter.
pub fn serde_with(expr: &TypeExpr) -> Option<&'static str> {
  match expr {
    TypeExpr::Bare("bytes") => Some("serde_with::bytes"),
    TypeExpr::Bare("int64") => Some("serde_with::int64"),
    TypeExpr::Vector(inner) if matches!(**inner, TypeExpr::Bare("int64")) => Some("serde_with::int64_vec"),
    _ => None,
  }
}

/// Formats a schema name as a Rust identifier, prefixing `r#` for any keyword.
pub fn escape_keyword(ident: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    if RUST_KEYWORDS.binary_search(&ident).is_ok() {
      f.write_str("r#")?;
    }
    f.write_str(ident)
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
