use std::fmt;

use td_parser::{Combinator, Definition, DefinitionKind, Field, TypeExpr};

use crate::scc::SccMap;
use crate::utils;

const DERIVES: &str = "Debug, Clone, PartialEq";

pub fn generate(ast: &[Definition]) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let scc = SccMap::from_ast(ast);

    let mut enums: Vec<_> = ast.iter().map(|d| d.comb.category).collect();
    enums.sort_unstable();
    enums.dedup();

    let mut types = Vec::default();
    let mut fns = Vec::default();
    for d in ast {
      match d.kind {
        DefinitionKind::Type => types.push(&d.comb),
        DefinitionKind::Function => fns.push(&d.comb),
      }
    }
    types.sort_unstable_by_key(|c| (c.category, c.name));

    let groups: Vec<_> = types
      .chunk_by(|a, b| a.category == b.category)
      .filter_map(|g| {
        let &[c, ..] = g else { return None };
        let None = utils::tl_type_to_rust(c.category) else { return None };
        Some((c.category, g))
      })
      .collect();

    writeln!(f, "pub mod types {{")?;
    write!(f, "{}", types_mod(&groups, &enums, &scc))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "pub mod enums {{")?;
    write!(f, "{}", enums_mod(&groups))?;
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "pub mod fns {{")?;
    write!(f, "{}", fns_mod(&fns, &enums, &scc))?;
    writeln!(f, "}}")
  })
}

fn types_mod(groups: &[(&str, &[&Combinator])], enums: &[&str], scc: &SccMap) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use serde::{{Deserialize, Serialize}};", "")?;
    writeln!(f, "{:2}use crate::serde_with;", "")?;
    writeln!(f, "{:2}use super::enums;", "")?;

    for &(_, group) in groups {
      for c in group {
        let [_, ..] = &*c.fields else { continue };
        writeln!(f)?;
        writeln!(f, "{}", r#struct(c, enums, scc, false))?;
      }
    }

    Ok(())
  })
}

fn enums_mod(groups: &[(&str, &[&Combinator])]) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use serde::{{Deserialize, Serialize}};", "")?;
    writeln!(f, "{:2}use super::types;", "")?;

    for &(category, group) in groups {
      writeln!(f)?;
      writeln!(f, "{}", r#enum(category, group))?;
    }

    Ok(())
  })
}

fn r#enum(category: &str, group: &[&Combinator]) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let has_unit_default = matches!(group, &[first, ..] if first.fields.is_empty());
    let derive_default = if has_unit_default { ", Default" } else { "" };
    let derive_serde = ", Serialize, Deserialize";
    let class_doc = group.iter().find_map(|c| c.class);

    write!(f, "{:2}", doc_comment(class_doc))?;
    writeln!(f, "{:2}#[derive({DERIVES}{derive_default}{derive_serde})]", "")?;
    writeln!(f, r#"{:2}#[serde(tag = "@type")]"#, "")?;
    writeln!(f, "{:2}pub enum {category} {{", "")?;

    for (i, c) in group.iter().enumerate() {
      let name = utils::escape_rust_keyword(c.name);
      if let Some(_) = class_doc {
        write!(f, "{:4}", doc_comment(c.desc))?;
      }
      if i == 0 && has_unit_default {
        writeln!(f, "{:4}#[default]", "")?;
      }
      let ty = match c.fields.len() {
        0 => format_args!(""),
        _ => format_args!("(types::{name})"),
      };
      writeln!(f, "{:4}{name}{ty},", "")?;
    }

    if !has_unit_default && let [first, ..] = group {
      let name = utils::escape_rust_keyword(first.name);
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

fn fns_mod(fns: &[&Combinator], enums: &[&str], scc: &SccMap) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    writeln!(f, "{:2}use serde::Serialize;", "")?;
    writeln!(f, "{:2}use crate::{{serde_with, traits::Function}};", "")?;
    writeln!(f, "{:2}use super::{{enums, types}};", "")?;

    for c in fns {
      writeln!(f)?;
      writeln!(f, "{}", r#fn(c, enums, scc))?;
    }

    Ok(())
  })
}

fn r#fn(c: &Combinator, enums: &[&str], scc: &SccMap) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let name = utils::escape_rust_keyword(c.name);
    let (ret_path, ret_ty) = match utils::tl_type_to_rust(c.category) {
      Some(name) => ("", name),
      None => ("enums::", c.category),
    };

    writeln!(f, "{}", r#struct(c, enums, scc, true))?;
    writeln!(f)?;
    writeln!(f, "{:2}impl Function for {name} {{", "")?;
    writeln!(f, "{:4}type Return = {ret_path}{ret_ty};", "")?;
    write!(f, "{:2}}}", "")
  })
}

fn r#struct(c: &Combinator, enums: &[&str], scc: &SccMap, is_fn: bool) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let serde_derive = if is_fn { "Serialize" } else { "Serialize, Deserialize" };
    let serde_args = if is_fn { r#"tag = "@type""# } else { "default" };

    write!(f, "{:2}", doc_comment(c.desc))?;
    writeln!(f, "{:2}#[derive({DERIVES}, Default, {serde_derive})]", "")?;
    writeln!(f, "{:2}#[serde({serde_args})]", "")?;
    writeln!(f, "{:2}pub struct {} {{", "", utils::escape_rust_keyword(c.name))?;
    for f_field in &c.fields {
      writeln!(f, "{}", field(f_field, enums, scc, is_fn, c.name))?;
    }
    write!(f, "{:2}}}", "")
  })
}

fn field(field: &Field, enums: &[&str], scc: &SccMap, is_fn: bool, struct_name: &str) -> impl fmt::Display {
  fn is_bytes(expr: &TypeExpr) -> bool {
    matches!(expr, TypeExpr::Bare("bytes"))
  }

  fn is_int64(expr: &TypeExpr) -> bool {
    matches!(expr, TypeExpr::Bare("int64"))
  }

  fn is_int64_vec(expr: &TypeExpr) -> bool {
    matches!(expr, TypeExpr::Vector(inner) if is_int64(inner))
  }

  fmt::from_fn(move |f| {
    write!(f, "{:4}", doc_comment(field.desc))?;
    let serde_args = match field {
      // f if f.is_optional => Some(r#"skip_serializing_if = "Option::is_none""#),
      f if is_bytes(&f.type_expr) => Some(r#"with = "serde_with::bytes""#),
      f if is_int64(&f.type_expr) => Some(r#"with = "serde_with::int64""#),
      f if is_int64_vec(&f.type_expr) => Some(r#"with = "serde_with::int64_vec""#),
      _ => None,
    };
    if let Some(args) = serde_args {
      writeln!(f, "{:4}#[serde({args})]", "")?;
    }
    let name = utils::escape_rust_keyword(field.name);
    let ty = type_expr(&field.type_expr, enums, Some(scc), is_fn, struct_name);
    let ty = if field.is_optional { format_args!("Option<{ty}>") } else { format_args!("{ty}") };
    write!(f, "{:4}pub {name}: {ty},", "")
  })
}

fn type_expr(expr: &TypeExpr, enums: &[&str], scc: Option<&SccMap>, is_fn: bool, struct_name: &str) -> impl fmt::Display {
  fmt::from_fn(move |f| {
    let struct_prefix = if is_fn { "types::" } else { "self::" };
    let enum_prefix = "enums::";

    match expr {
      TypeExpr::Bare(name) if let Some(name) = utils::tl_type_to_rust(name) => f.write_str(name),
      TypeExpr::Bare(name) if let Ok(_) = enums.binary_search(name) => match scc {
        Some(scc) if scc.in_same_scc(struct_name, name) => write!(f, "Box<{enum_prefix}{name}>"),
        _ => write!(f, "{enum_prefix}{name}"),
      },
      TypeExpr::Bare(name) => write!(f, "{struct_prefix}{name}"),
      TypeExpr::Vector(inner) => {
        write!(f, "Vec<{}>", type_expr(inner, enums, None, is_fn, ""))
      }
    }
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
