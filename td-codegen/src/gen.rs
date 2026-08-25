//! Formatting of the generated `types`, `enums`, and `fns` modules.
//!
//! Every helper returns a borrowed [`fmt::Display`] adapter. This keeps the
//! generator a single streaming pass after [`SchemaIndex`] builds its sorted indexes
//! and recursive-layout map.

use std::fmt::{self, Write};
use std::time::Instant;

use td_parser::{Combinator, Definition, Field, TypeExpr};

use crate::ctx::SchemaIndex as Context;
use crate::util;

const DERIVES: &str = "Debug, Clone, PartialEq";

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
  let body = format(&ast);
  let mut out = String::default();
  let _ = write!(out, "{body}");
  let t2 = Instant::now();

  let [parse, codegen] = [[t0, t1], [t1, t2]].map(|[a, b]| b.duration_since(a));
  let header = crate::header(schema, [parse, codegen]).to_string();
  if !header.is_empty() {
    out.insert_str(0, &header);
  }

  Ok(out)
}

/// Formats Rust modules for the supplied parsed schema AST.
///
/// The result borrows `ast` and writes generated source when formatted. Object
/// constructors with fields become structs under `types`; result categories
/// become internally tagged enums under `enums`; functions become request
/// structs under `fns` and implement `td_types::traits::Function` through the
/// generated crate context.
///
/// Types are grouped and sorted for deterministic output. Functions retain schema
/// order. Documentation from the parsed schema is emitted on the corresponding
/// structs, variants, fields, and requests.
pub fn format(ast: &[Definition<'_>]) -> impl fmt::Display {
  let ctx = Context::new(ast);

  fmt::from_fn(move |f| {
    writeln!(f, "/// Concrete payloads carried by `TDLib` object constructors.")?;
    writeln!(f, "///")?;
    writeln!(f, "/// Unit constructors appear only as variants in [`enums`].")?;
    writeln!(f, "pub mod types {{")?;
    write!(f, "{}", types_mod(&ctx))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "/// Polymorphic `TDLib` objects grouped by their TL result type.")?;
    writeln!(f, "///")?;
    writeln!(f, "/// Enums use the JSON `@type` field to select a constructor payload from [`types`].")?;
    writeln!(f, "pub mod enums {{")?;
    write!(f, "{}", enums_mod(&ctx))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "/// Serializable `TDLib` requests and their typed response associations.")?;
    writeln!(f, "///")?;
    writeln!(f, "/// Each request implements [`crate::traits::Function`].")?;
    writeln!(f, "pub mod fns {{")?;
    write!(f, "{}", fns_mod(&ctx))?;
    writeln!(f, "}}")
  })
}

fn types_mod(ctx: &Context) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use crate::prelude::*;", "")?;

    for (_, group) in ctx.ctor_groups() {
      for c in group {
        let 1.. = c.fields.len() else { continue };
        writeln!(f)?;
        writeln!(f, "{}", r#struct(ctx, c, false))?;
      }
    }

    Ok(())
  })
}

fn enums_mod(ctx: &Context) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use crate::prelude::*;", "")?;

    for (name, items) in ctx.ctor_groups() {
      writeln!(f)?;
      writeln!(f, "{}", r#enum(name, items))?;
    }

    Ok(())
  })
}

fn fns_mod(ctx: &Context) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use crate::prelude::*;", "")?;

    for comb in ctx.fns() {
      writeln!(f)?;
      writeln!(f, "{}", r#fn(ctx, comb))?;
    }

    Ok(())
  })
}

fn r#enum(enum_name: &str, items: &[&Combinator]) -> impl fmt::Display {
  let group_desc = items.iter().find_map(|c| c.meta);
  let has_unit_default = matches!(items, &[first, ..] if first.fields.is_empty());
  let default_derive = if has_unit_default { ", Default" } else { "" };
  let serde_derives = ", Serialize, Deserialize";
  let serde_args = r#"tag = "@type""#;

  fmt::from_fn(move |f| {
    write!(f, "{:2}", doc_comment(group_desc))?;
    writeln!(f, "{:2}#[derive({DERIVES}{default_derive}{serde_derives})]", "")?;
    writeln!(f, "{:2}#[serde({serde_args})]", "")?;
    writeln!(f, "{:2}pub enum {enum_name} {{", "")?;

    for (i, comb) in items.iter().enumerate() {
      if let Some(_) = group_desc {
        write!(f, "{:4}", doc_comment(comb.desc))?;
      }
      if has_unit_default && let 0 = i {
        writeln!(f, "{:4}#[default]", "")?;
      }
      let name = util::escaped_keyword(comb.name);
      let fields = match comb.fields.len() {
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
      let first = util::escaped_keyword(first.name);
      writeln!(f)?;
      writeln!(f)?;
      writeln!(f, "{:2}impl Default for {enum_name} {{", "")?;
      writeln!(f, "{:4}fn default() -> Self {{", "")?;
      writeln!(f, "{:6}types::{first}::default().into()", "")?;
      writeln!(f, "{:4}}}", "")?;
      write!(f, "{:2}}}", "")?;
    }

    for comb in items {
      let 1.. = comb.fields.len() else { continue };
      let name = util::escaped_keyword(comb.name);
      writeln!(f)?;
      writeln!(f)?;
      writeln!(f, "{:2}impl From<types::{name}> for {enum_name} {{", "")?;
      writeln!(f, "{:4}fn from(value: types::{name}) -> Self {{", "")?;
      writeln!(f, "{:6}Self::{name}(value)", "")?;
      writeln!(f, "{:4}}}", "")?;
      write!(f, "{:2}}}", "")?;
    }

    Ok(())
  })
}

fn r#fn(ctx: &Context, comb: &Combinator) -> impl fmt::Display {
  let name = util::escaped_keyword(comb.name);
  let [ret_path, ret_type] = match util::to_native(comb.r#type) {
    Some(native) => ["", native],
    None => ["enums::", comb.r#type],
  };

  fmt::from_fn(move |f| {
    writeln!(f, "{}", r#struct(ctx, comb, true))?;
    writeln!(f)?;
    writeln!(f, "{:2}impl Function for {} {{", "", name)?;
    writeln!(f, "{:4}type Return = {ret_path}{ret_type};", "")?;
    write!(f, "{:2}}}", "")
  })
}

fn r#struct(ctx: &Context, comb: &Combinator, is_fn: bool) -> impl fmt::Display {
  let name = util::escaped_keyword(comb.name);
  let serde_derives = if is_fn { "Serialize" } else { "Serialize, Deserialize" };
  let serde_args = if is_fn { r#"tag = "@type""# } else { "default" };

  fmt::from_fn(move |f| {
    write!(f, "{:2}", doc_comment(comb.desc))?;
    writeln!(f, "{:2}#[derive({DERIVES}, Default, {serde_derives})]", "")?;
    writeln!(f, "{:2}#[serde({serde_args})]", "")?;
    writeln!(f, "{:2}pub struct {name} {{", "")?;
    for cf in &comb.fields {
      writeln!(f, "{}", field(ctx, cf, is_fn, comb.name))?;
    }
    write!(f, "{:2}}}", "")
  })
}

fn field(ctx: &Context, field: &Field, is_fn: bool, struct_name: &str) -> impl fmt::Display {
  let name = util::escaped_keyword(field.name);
  let expr = type_expr(ctx, &field.r#type, is_fn, struct_name);

  let serde_args = match &field.r#type {
    t if util::is_bytes(t) => Some(r#"with = "serde_with::bytes""#),
    t if util::is_int64(t) => Some(r#"with = "serde_with::int64""#),
    t if util::is_int64_vec(t) => Some(r#"with = "serde_with::int64_vec""#),
    _ => None,
  };

  fmt::from_fn(move |f| {
    write!(f, "{:4}", doc_comment(field.desc))?;
    if let Some(args) = serde_args {
      writeln!(f, "{:4}#[serde({args})]", "")?;
    }
    let [open, close] = if field.is_optional { ["Option<", ">"] } else { ["", ""] };
    write!(f, "{:4}pub {name}: {open}{expr}{close},", "")
  })
}

fn type_expr(ctx: &Context, expr: &TypeExpr, is_fn: bool, struct_name: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| match expr {
    TypeExpr::Bare(name) if let Some(name) = util::to_native(name) => f.write_str(name),
    TypeExpr::Bare(name) if ctx.is_enum(name) => {
      // Vector recursion calls this helper with no enclosing struct and is
      // already indirect. Direct recursive SCC edges are boxed conservatively.
      let [open, close] = if ctx.needs_box([struct_name, name]) { ["Box<", ">"] } else { ["", ""] };
      write!(f, "{open}enums::{name}{close}")
    }
    TypeExpr::Bare(name) => write!(f, "{}{name}", if is_fn { "types::" } else { "self::" }),
    TypeExpr::Vector(inner) => write!(f, "Vec<{}>", type_expr(ctx, inner, is_fn, "")),
  })
}

fn doc_comment(doc: Option<&str>) -> impl fmt::Display {
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
