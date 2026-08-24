//! Detection of recursive Rust layouts in the parsed TL type graph.
//!
//! Vertices are constructor names and result-category names. A non-unit enum
//! contributes an edge from its category to its constructor payload, and each
//! constructor contributes edges to directly embedded, non-native field types.
//! Vector element references are excluded because `Vec` already breaks the Rust
//! layout cycle. Constructors and categories in one strongly connected component
//! therefore identify precisely the enum fields that require `Box`.

use td_parser::{Combinator, Definition, DefinitionKind, TypeExpr};

use crate::{graph::Graph, util};

/// Sorted type names paired with their strongly connected component IDs.
#[derive(Debug)]
pub struct SccMap<'a> {
  /// Sorted vertex names used for binary-search lookup.
  names: Vec<&'a str>,
  /// Component ID parallel to each entry in `names`.
  ids: Vec<usize>,
}

impl<'a> SccMap<'a> {
  /// Builds the direct-layout dependency graph for all type definitions.
  pub fn from_ast(ast: &[Definition<'a>]) -> Self {
    let mut names: Vec<_> = ast //.
      .iter()
      .filter(|d| d.kind == DefinitionKind::Type)
      .flat_map(|d| [d.comb.name, d.comb.r#type])
      .collect();

    names.sort_unstable();
    names.dedup();

    // Category -> constructor models the enum payload. Constructor -> field
    // models direct storage. Unit constructors and vector fields add no storage
    // edge, so neither can create an infinite-size Rust type.
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

  /// Reports whether two known names belong to one recursive component.
  pub fn in_same_scc(&self, names: [&str; 2]) -> bool {
    let [Ok(i), Ok(j)] = names.map(|n| self.names.binary_search(&n)) else { return false };
    let [Some(&a), Some(&b)] = [i, j].map(|x| self.ids.get(x)) else { return false };
    a == b
  }
}

/// Returns a directly embedded non-native type name, excluding vector indirection.
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
