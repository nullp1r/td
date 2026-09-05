//! Formatting of the generated `types`, `enums`, and `fns` modules.
//!
//! Every helper returns a borrowed [`fmt::Display`] adapter. This keeps the
//! generator a single streaming pass after [`Schema`] indexes definition groups
//! and layout cycle detection.

use std::fmt;
use std::time::Instant;

use td_parser::{Definition as Def, DefinitionKind as Kind, Field, TypeExpr};

use crate::header;
use crate::schema::{self, Schema};

/// Compiles a `TDLib` API schema into a complete Rust source string with metadata header.
///
/// Parses `schema`, formats all `types`, `enums`, and `fns` modules, measures the
/// elapsed generation duration, and prepends the source header if present.
///
/// # Errors
///
/// Returns a parse error if `schema` contains invalid syntax.
pub fn compile(schema: &str) -> Result<String, td_parser::Error<'_>> {
  let t0 = Instant::now();
  let ast = td_parser::parse(schema)?;
  let t1 = Instant::now();
  let out = format(&ast).to_string();
  let t2 = Instant::now();

  let [parse, codegen] = [t1.duration_since(t0), t2.duration_since(t1)];
  let header = header::header(schema, [parse, codegen]).to_string();
  if !header.is_empty() {
    let mut final_out = String::with_capacity(header.len() + out.len());
    final_out.push_str(&header);
    final_out.push_str(&out);
    return Ok(final_out);
  }

  Ok(out)
}

/// Formats Rust modules for the supplied parsed schema AST.
///
/// The result borrows `ast` and writes generated source when formatted. Object
/// constructors with fields become structs under `types`; result types
/// become internally tagged enums under `enums`; functions become request
/// structs under `fns` and implement `td_types::traits::Function` through the
/// generated crate context.
///
/// Types are grouped and sorted for deterministic output. Functions retain schema
/// order. Documentation from the parsed schema is emitted on the corresponding
/// structs, variants, fields, and requests.
pub fn format(ast: &[Def<'_>]) -> impl fmt::Display {
  let schema = Schema::new(ast);

  fmt::from_fn(move |f| {
    writeln!(f, "/// Concrete payloads carried by `TDLib` object constructors.")?;
    writeln!(f, "///")?;
    writeln!(f, "/// Unit constructors appear only as variants in [`enums`].")?;
    writeln!(f, "pub mod types {{")?;
    write!(f, "{}", fmt_types(&schema))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "/// Polymorphic `TDLib` objects grouped by their TL result type.")?;
    writeln!(f, "///")?;
    writeln!(f, "/// Enums use the JSON `@type` field to select a constructor payload from [`types`].")?;
    writeln!(f, "pub mod enums {{")?;
    write!(f, "{}", fmt_enums(&schema))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "/// Serializable `TDLib` requests and their typed response associations.")?;
    writeln!(f, "///")?;
    writeln!(f, "/// Each request implements [`crate::traits::Function`].")?;
    writeln!(f, "pub mod fns {{")?;
    write!(f, "{}", fmt_fns(&schema))?;
    writeln!(f, "}}")
  })
}

fn fmt_types(schema: &Schema) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use crate::prelude::*;", "")?;

    for (_, ctors) in schema.types() {
      for ctor in ctors {
        let 1.. = ctor.fields.len() else { continue };
        writeln!(f)?;
        writeln!(f, "{}", fmt_struct(schema, ctor, Kind::Type))?;
      }
    }

    Ok(())
  })
}

fn fmt_enums(schema: &Schema) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use crate::prelude::*;", "")?;

    for (type_name, ctors) in schema.types() {
      writeln!(f)?;
      writeln!(f, "{}", fmt_enum(type_name, ctors))?;
    }

    Ok(())
  })
}

fn fmt_fns(schema: &Schema) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use crate::prelude::*;", "")?;

    for &r#fn in schema.fns() {
      writeln!(f)?;
      writeln!(f, "{}", fmt_fn(schema, r#fn))?;
    }

    Ok(())
  })
}

fn fmt_enum(type_name: &str, items: &[&Def]) -> impl fmt::Display {
  let group_desc = items.iter().find_map(|c| c.meta);
  let has_unit_default = matches!(items, &[first, ..] if first.fields.is_empty());
  let default: &[&str] = if has_unit_default { &["Default"] } else { &[] };
  let serde = &["Serialize", "Deserialize"];
  let serde_args = r#"tag = "@type""#;

  fmt::from_fn(move |f| {
    write!(f, "{:2}", fmt_doc_comment(group_desc))?;
    write!(f, "{:2}", fmt_derive(&[&["Debug", "Clone", "PartialEq"], default, serde]))?;
    writeln!(f, "{:2}#[serde({serde_args})]", "")?;
    writeln!(f, "{:2}pub enum {type_name} {{", "")?;

    for (i, ctor) in items.iter().enumerate() {
      if let Some(_) = group_desc {
        write!(f, "{:4}", fmt_doc_comment(ctor.desc))?;
      }
      if has_unit_default && let 0 = i {
        writeln!(f, "{:4}#[default]", "")?;
      }
      let name = schema::escape_keyword(ctor.name);
      let fields = match ctor.fields.len() {
        0 => format_args!(""),
        _ => format_args!("(types::{name})"),
      };
      writeln!(f, "{:4}{name}{fields},", "")?;
    }
    write!(f, "{:2}}}", "")?;

    // `Default` is a construction convenience rather than a TDLib semantic
    // default. Select the first schema constructor and keep unit variants
    // derivable where Rust permits it.
    if !has_unit_default && let [first, ..] = items {
      let first = schema::escape_keyword(first.name);
      writeln!(f)?;
      writeln!(f)?;
      writeln!(f, "{:2}impl Default for {type_name} {{", "")?;
      writeln!(f, "{:4}fn default() -> Self {{", "")?;
      writeln!(f, "{:6}types::{first}::default().into()", "")?;
      writeln!(f, "{:4}}}", "")?;
      write!(f, "{:2}}}", "")?;
    }

    for ctor in items {
      let 1.. = ctor.fields.len() else { continue };
      let name = schema::escape_keyword(ctor.name);
      writeln!(f)?;
      writeln!(f)?;
      writeln!(f, "{:2}impl From<types::{name}> for {type_name} {{", "")?;
      writeln!(f, "{:4}fn from(value: types::{name}) -> Self {{", "")?;
      writeln!(f, "{:6}Self::{name}(value)", "")?;
      writeln!(f, "{:4}}}", "")?;
      write!(f, "{:2}}}", "")?;
    }

    Ok(())
  })
}

fn fmt_fn(schema: &Schema, def: &Def) -> impl fmt::Display {
  let name = schema::escape_keyword(def.name);
  let [ret_path, ret_type] = match schema::primitive(def.r#type) {
    Some(primitive) => ["", primitive],
    None => ["enums::", def.r#type],
  };

  fmt::from_fn(move |f| {
    writeln!(f, "{}", fmt_struct(schema, def, Kind::Function))?;
    writeln!(f)?;
    writeln!(f, "{:2}impl Function for {name} {{", "")?;
    writeln!(f, "{:4}type Return = {ret_path}{ret_type};", "")?;
    write!(f, "{:2}}}", "")
  })
}

fn fmt_struct(schema: &Schema, def: &Def, kind: Kind) -> impl fmt::Display {
  let name = schema::escape_keyword(def.name);
  let (serde, serde_args): (&[&str], _) = match kind {
    Kind::Function => (&["Serialize"], r#"tag = "@type""#),
    Kind::Type => (&["Serialize", "Deserialize"], "default"),
  };

  fmt::from_fn(move |f| {
    write!(f, "{:2}", fmt_doc_comment(def.desc))?;
    write!(f, "{:2}", fmt_derive(&[&["Debug", "Clone", "PartialEq", "Default"], serde]))?;
    writeln!(f, "{:2}#[serde({serde_args})]", "")?;
    writeln!(f, "{:2}pub struct {name} {{", "")?;
    for cf in &def.fields {
      writeln!(f, "{}", fmt_field(schema, cf, kind, def.name))?;
    }
    write!(f, "{:2}}}", "")
  })
}

fn fmt_field(schema: &Schema, field: &Field, kind: Kind, parent: &str) -> impl fmt::Display {
  let name = schema::escape_keyword(field.name);
  let expr = fmt_type_expr(schema, &field.r#type, kind, parent);
  let serde_args = schema::serde_with(&field.r#type);

  fmt::from_fn(move |f| {
    write!(f, "{:4}", fmt_doc_comment(field.desc))?;
    if let Some(args) = serde_args {
      writeln!(f, "{:4}#[serde(with = \"{args}\")]", "")?;
    }
    let [open, close] = if field.is_optional { ["Option<", ">"] } else { ["", ""] };
    write!(f, "{:4}pub {name}: {open}{expr}{close},", "")
  })
}

fn fmt_type_expr(schema: &Schema, expr: &TypeExpr, kind: Kind, parent: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| match expr {
    TypeExpr::Bare(name) if let Some(primitive) = schema::primitive(name) => f.write_str(primitive),
    TypeExpr::Bare(name) if schema.is_type(name) => {
      // Vector recursion calls this helper with no enclosing struct and is
      // already indirect. Direct recursive SCC edges are boxed conservatively.
      let [open, close] = if schema.is_recursive(parent, name) { ["Box<", ">"] } else { ["", ""] };
      write!(f, "{open}enums::{name}{close}")
    }
    TypeExpr::Bare(name) => {
      let prefix = match kind {
        Kind::Function => "types::",
        Kind::Type => "self::",
      };
      write!(f, "{prefix}{name}")
    }
    TypeExpr::Vector(inner) => write!(f, "Vec<{}>", fmt_type_expr(schema, inner, kind, "")),
  })
}

fn fmt_doc_comment(doc: Option<&str>) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    if let Some(doc) = doc {
      let pad = f.width().unwrap_or_default();
      // TDLib uses `//-` for a continuation that should become a new rustdoc line.
      for line in doc.split("\n//-") {
        writeln!(f, "{:pad$}/// {line}", "")?;
      }
    }
    Ok(())
  })
}

fn fmt_derive(groups: &[&[&str]]) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let mut derives = groups.iter().flat_map(|&g| g);
    let Some(first) = derives.next() else { return Ok(()) };
    let pad = f.width().unwrap_or_default();
    write!(f, "{:pad$}#[derive({first}", "")?;
    for item in derives {
      write!(f, ", {item}")?;
    }
    writeln!(f, ")]")
  })
}
