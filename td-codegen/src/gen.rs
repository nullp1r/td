use std::fmt;

use td_parser::{Combinator, Definition, Field, TypeExpr};

use crate::ctx::Context;
use crate::util;

const DERIVES: &str = "Debug, Clone, PartialEq";

pub fn generate(ast: &[Definition]) -> impl fmt::Display {
  let ctx = Context::new(ast);

  fmt::from_fn(move |f| {
    writeln!(f, "pub mod types {{")?;
    write!(f, "{}", types_mod(&ctx))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "pub mod enums {{")?;
    write!(f, "{}", enums_mod(&ctx))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "pub mod fns {{")?;
    write!(f, "{}", fns_mod(&ctx))?;
    writeln!(f, "}}")
  })
}

fn types_mod(ctx: &Context) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use serde::{{Deserialize, Serialize}};", "")?;
    writeln!(f, "{:2}use crate::serde_with;", "")?;
    writeln!(f, "{:2}use super::enums;", "")?;

    for (_, group) in ctx.groups() {
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
    writeln!(f, "{:2}use serde::{{Deserialize, Serialize}};", "")?;
    writeln!(f, "{:2}use super::types;", "")?;

    for (name, items) in ctx.groups() {
      writeln!(f)?;
      writeln!(f, "{}", r#enum(name, items))?;
    }

    Ok(())
  })
}

fn fns_mod(ctx: &Context) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use serde::Serialize;", "")?;
    writeln!(f, "{:2}use crate::{{serde_with, traits::Function}};", "")?;
    writeln!(f, "{:2}use super::{{enums, types}};", "")?;

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
      let [open, close] = if ctx.in_same_scc([struct_name, name]) { ["Box<", ">"] } else { ["", ""] };
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
      for line in doc.split("\n//-") {
        writeln!(f, "{:pad$}/// {line}", "")?;
      }
    }
    Ok(())
  })
}
