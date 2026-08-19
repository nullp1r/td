//! SCC (Strongly Connected Component) analysis for TD type definitions.
//! Used to detect cyclic type references that need `Box<>` wrapping
//! in generated Rust code.

use td_parser::{Definition, DefinitionKind, TypeExpr};

use crate::{graph::CsrGraph, util};

/// Maps type names to their SCC group ID.
///
/// Two types in the same SCC are mutually recursive and any reference from one
/// to the other must be wrapped in [`Box`] to break the cycle.
#[derive(Debug)]
pub struct SccMap<'a> {
  names: Vec<&'a str>,
  ids: Vec<usize>,
}

impl<'a> SccMap<'a> {
  /// Builds the SCC map from parsed AST definitions.
  pub fn from_ast(ast: &[Definition<'a>]) -> Self {
    let mut names: Vec<_> = ast
      .iter() //.
      .filter(|d| d.kind == DefinitionKind::Type)
      .flat_map(|d| [d.comb.name, d.comb.category])
      .collect();

    names.sort_unstable();
    names.dedup();

    let mut edges = Vec::new();
    for def in ast.iter().filter(|d| d.kind == DefinitionKind::Type) {
      if let Ok(src) = names.binary_search(&def.comb.name)
        && let Ok(cat) = names.binary_search(&def.comb.category)
      {
        if !def.comb.fields.is_empty() && cat != src {
          edges.push([cat, src]);
        }
        for field in &def.comb.fields {
          if let Some(name) = used_type(&field.type_expr)
            && let Ok(dst) = names.binary_search(&name)
          {
            edges.push([src, dst]);
          }
        }
      }
    }

    let graph = CsrGraph::from_pairs(edges, names.len());
    let ids = graph.scc();
    Self { names, ids }
  }

  /// Looks up the SCC group ID for a type name, if present.
  pub fn get(&self, name: &str) -> Option<usize> {
    self.names.binary_search(&name).ok().map(|i| self.ids[i])
  }

  /// Returns `true` if both types are registered and belong to the same SCC group.
  pub fn in_same_scc(&self, [a, b]: [&str; 2]) -> bool {
    matches!([self.get(a), self.get(b)], [Some(a), Some(b)] if a == b)
  }
}

/// Extract a user-defined type name from a [`TypeExpr`], unwrapping any nested
/// [`TypeExpr::Vector`] and skipping Rust-native primitive types.
fn used_type<'a>(mut expr: &TypeExpr<'a>) -> Option<&'a str> {
  while let TypeExpr::Vector(inner) = expr {
    expr = inner;
  }
  match expr {
    &TypeExpr::Bare(name) if let None = util::to_native(name) => Some(name),
    _ => None,
  }
}
