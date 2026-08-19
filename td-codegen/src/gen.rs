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

    for (category, group) in ctx.groups() {
      writeln!(f)?;
      writeln!(f, "{}", r#enum(category, group))?;
    }

    Ok(())
  })
}

fn r#enum(category: &str, group: &[&Combinator]) -> impl fmt::Display {
  let has_unit_default = matches!(group, &[first, ..] if first.fields.is_empty());
  let derive_default = if has_unit_default { ", Default" } else { "" };
  let derive_serde = ", Serialize, Deserialize";
  let class_doc = group.iter().find_map(|c| c.class);

  fmt::from_fn(move |f| {
    write!(f, "{:2}", doc_comment(class_doc))?;
    writeln!(f, "{:2}#[derive({DERIVES}{derive_default}{derive_serde})]", "")?;
    writeln!(f, r#"{:2}#[serde(tag = "@type")]"#, "")?;
    writeln!(f, "{:2}pub enum {category} {{", "")?;

    for (i, c) in group.iter().enumerate() {
      let name = util::escaped_keyword(c.name);
      if let Some(_) = class_doc {
        write!(f, "{:4}", doc_comment(c.desc))?;
      }
      if has_unit_default && let 0 = i {
        writeln!(f, "{:4}#[default]", "")?;
      }
      let ty = match c.fields.len() {
        0 => format_args!(""),
        _ => format_args!("(types::{name})"),
      };
      writeln!(f, "{:4}{name}{ty},", "")?;
    }

    if !has_unit_default && let [first, ..] = group {
      let name = util::escaped_keyword(first.name);
      writeln!(f, "{:2}}}", "")?;
      writeln!(f)?;
      writeln!(f, "{:2}impl Default for {category} {{", "")?;
      writeln!(f, "{:4}fn default() -> Self {{", "")?;
      writeln!(f, "{:6}Self::{name}(types::{name}::default())", "")?;
      writeln!(f, "{:4}}}", "")?;
    }
    write!(f, "{:2}}}", "")
  })
}

fn fns_mod(ctx: &Context) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use serde::Serialize;", "")?;
    writeln!(f, "{:2}use crate::{{serde_with, traits::Function}};", "")?;
    writeln!(f, "{:2}use super::{{enums, types}};", "")?;

    for c in ctx.fns() {
      writeln!(f)?;
      writeln!(f, "{}", r#fn(ctx, c))?;
    }

    Ok(())
  })
}

fn r#fn(ctx: &Context, comb: &Combinator) -> impl fmt::Display {
  let name = util::escaped_keyword(comb.name);
  let [ret_path, ret_ty] = match util::to_native(comb.category) {
    Some(native) => ["", native],
    None => ["enums::", comb.category],
  };

  fmt::from_fn(move |f| {
    writeln!(f, "{}", r#struct(ctx, comb, true))?;
    writeln!(f)?;
    writeln!(f, "{:2}impl Function for {name} {{", "")?;
    writeln!(f, "{:4}type Return = {ret_path}{ret_ty};", "")?;
    write!(f, "{:2}}}", "")
  })
}

fn r#struct(ctx: &Context, comb: &Combinator, is_fn: bool) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let serde_derive = if is_fn { "Serialize" } else { "Serialize, Deserialize" };
    let serde_args = if is_fn { r#"tag = "@type""# } else { "default" };

    write!(f, "{:2}", doc_comment(comb.desc))?;
    writeln!(f, "{:2}#[derive({DERIVES}, Default, {serde_derive})]", "")?;
    writeln!(f, "{:2}#[serde({serde_args})]", "")?;
    writeln!(f, "{:2}pub struct {} {{", "", util::escaped_keyword(comb.name))?;
    for f_field in &comb.fields {
      writeln!(f, "{}", field(ctx, f_field, is_fn, comb.name))?;
    }
    write!(f, "{:2}}}", "")
  })
}

fn field(ctx: &Context, field: &Field, is_fn: bool, struct_name: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    write!(f, "{:4}", doc_comment(field.desc))?;
    let serde_args = match &field.type_expr {
      expr if util::is_bytes(expr) => Some(r#"with = "serde_with::bytes""#),
      expr if util::is_int64(expr) => Some(r#"with = "serde_with::int64""#),
      expr if util::is_int64_vec(expr) => Some(r#"with = "serde_with::int64_vec""#),
      _ => None,
    };
    if let Some(args) = serde_args {
      writeln!(f, "{:4}#[serde({args})]", "")?;
    }
    let name = util::escaped_keyword(field.name);
    let ty = type_expr(ctx, &field.type_expr, is_fn, struct_name);
    let ty = if field.is_optional { format_args!("Option<{ty}>") } else { format_args!("{ty}") };
    write!(f, "{:4}pub {name}: {ty},", "")
  })
}

fn type_expr(ctx: &Context, expr: &TypeExpr, is_fn: bool, struct_name: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| match expr {
    TypeExpr::Bare(name) if let Some(name) = util::to_native(name) => f.write_str(name),
    TypeExpr::Bare(name) if ctx.is_enum(name) => {
      let [lhs, rhs] = if ctx.in_same_scc([struct_name, name]) { ["Box<", ">"] } else { ["", ""] };
      write!(f, "{lhs}enums::{name}{rhs}")
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
