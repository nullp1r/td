//! Strongly Connected Component (SCC) analysis for `TDLib` type definitions.
//!
//! Detects mutually recursive type references that require `Box<...>` indirection
//! to prevent infinite struct layout size in Rust.

use td_parser::{Combinator, Definition, DefinitionKind, TypeExpr};

use crate::{graph::Graph, util};

/// Maps type names to their strongly connected component ID.
///
/// Types sharing the same component ID form a recursive cycle and require boxing.
#[derive(Debug)]
pub struct SccMap<'a> {
  /// Sorted list of unique type and category names for binary search lookups.
  names: Vec<&'a str>,
  /// SCC component ID corresponding to each entry in `names`.
  ids: Vec<usize>,
}

impl<'a> SccMap<'a> {
  /// Builds the SCC mapping from parsed AST type definitions.
  pub fn from_ast(ast: &[Definition<'a>]) -> Self {
    let mut names: Vec<_> = ast //.
      .iter()
      .filter(|d| d.kind == DefinitionKind::Type)
      .flat_map(|d| [d.comb.name, d.comb.r#type])
      .collect();

    names.sort_unstable();
    names.dedup();

    let edges = ast
      .iter()
      .filter(|d| d.kind == DefinitionKind::Type)
      .flat_map(|d| {
        let Combinator { r#type, name, ref fields, .. } = d.comb;
        let cat = (!fields.is_empty() && r#type != name).then_some([r#type, name]);
        let fields = fields.iter().filter_map(move |f| Some([name, bare(&f.r#type)?]));
        cat.into_iter().chain(fields)
      })
      .filter_map(|pair| match pair.map(|n| names.binary_search(&n)) {
        [Ok(src), Ok(dst)] => Some([src, dst]),
        _ => None,
      })
      .collect();

    let ids = Graph::from_edges(edges, names.len()).scc();
    Self { names, ids }
  }

  /// Returns the SCC component ID for `name`, if registered.
  pub fn get(&self, name: &str) -> Option<usize> {
    self.names.binary_search(&name).ok().map(|i| self.ids[i])
  }

  /// Returns `true` if both types belong to the same recursive SCC component.
  pub fn in_same_scc(&self, [a, b]: [&str; 2]) -> bool {
    matches!([self.get(a), self.get(b)], [Some(a), Some(b)] if a == b)
  }
}

/// Extracts a directly embedded custom type name from `expr`, skipping native
/// primitives and `Vector`s (which allocate on heap and break layout cycles).
fn bare<'a>(expr: &TypeExpr<'a>) -> Option<&'a str> {
  match expr {
    &TypeExpr::Bare(inner) if let None = util::to_native(inner) => Some(inner),
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

  #[test]
  fn vector_breaks_cycle() {
    let input = "
      folder id:int32 subfolders:vector<Category> = Category;
    ";
    let ast = parse(input).unwrap();
    let scc = SccMap::from_ast(&ast);

    // Vector provides heap indirection, breaking the struct layout cycle.
    assert!(!scc.in_same_scc(["Category", "folder"]));
  }
}
