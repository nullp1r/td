//! Strongly Connected Component (SCC) analysis for TDLib type definitions.
//!
//! Detects mutually recursive type references that require `Box<...>` indirection
//! to prevent infinite struct layout size in Rust.

use td_parser::{Definition, DefinitionKind, TypeExpr};

use crate::{graph::CsrGraph, util};

/// Maps type names to their SCC group ID.
///
/// Types sharing the same group ID form a recursive cycle and require boxing.
#[derive(Debug)]
pub struct SccMap<'a> {
  /// Sorted list of unique type and category names for binary search lookups.
  names: Vec<&'a str>,
  /// SCC group ID corresponding to each name in `names`.
  ids: Vec<usize>,
}

impl<'a> SccMap<'a> {
  /// Builds the SCC mapping from parsed AST type definitions.
  pub fn from_ast(ast: &[Definition<'a>]) -> Self {
    let mut names: Vec<_> = ast //.
      .iter()
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
        // Add dependency edge from category (enum) to constructor (variant) if fields exist.
        if !def.comb.fields.is_empty() && cat != src {
          edges.push([cat, src]);
        }
        // Add dependency edge from constructor to each referenced field type.
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

  /// Returns the SCC group ID for `name`, if registered.
  pub fn get(&self, name: &str) -> Option<usize> {
    self.names.binary_search(&name).ok().map(|i| self.ids[i])
  }

  /// Returns `true` if both types belong to the same recursive SCC group.
  pub fn in_same_scc(&self, [a, b]: [&str; 2]) -> bool {
    matches!([self.get(a), self.get(b)], [Some(a), Some(b)] if a == b)
  }
}

/// Extracts a user-defined type name from `expr`, unwrapping any outer `Vector`s
/// and skipping native primitives.
fn used_type<'a>(mut expr: &TypeExpr<'a>) -> Option<&'a str> {
  while let TypeExpr::Vector(inner) = expr {
    expr = inner;
  }
  match expr {
    &TypeExpr::Bare(name) if util::to_native(name).is_none() => Some(name),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use td_parser::parse;

  use super::*;

  #[test]
  fn non_recursive_types() {
    let input = "
      user id:int32 name:string = User;
      message id:int32 author:User = Message;
    ";
    let ast = parse(input).unwrap();
    let scc = SccMap::from_ast(&ast);

    // User and Message are not mutually recursive.
    assert!(!scc.in_same_scc(["User", "Message"]));
    assert!(!scc.in_same_scc(["user", "message"]));
  }

  #[test]
  fn recursive_cycle_detection() {
    let input = "
      nodeLeaf value:int32 = TreeNode;
      nodeBranch left:TreeNode right:TreeNode = TreeNode;
    ";
    let ast = parse(input).unwrap();
    let scc = SccMap::from_ast(&ast);

    // TreeNode category contains nodeBranch which references TreeNode -> cycle.
    assert!(scc.in_same_scc(["TreeNode", "nodeBranch"]));
  }
}
