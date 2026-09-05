//! Conservative detection of recursive Rust layouts in a TL type graph.
//!
//! Vertices represent constructor structs and result types. Each non-unit type
//! contributes an edge to its constructor payload, and each constructor contributes
//! edges to directly stored, non-primitive field types. `Vec` fields are omitted
//! because their allocation already breaks recursion.
//!
//! Every direct field edge inside a strongly connected component is boxed. This
//! linear-time rule can box more fields than strictly necessary, but guarantees
//! finite layouts without solving the NP-hard minimum feedback-edge problem.

use td_parser::{Definition, DefinitionKind, TypeExpr};

use crate::graph::Graph;
use crate::schema;

/// Sorted type names and their recursive-layout component IDs.
#[derive(Debug)]
pub struct Layout<'a> {
  names: Vec<&'a str>,
  components: Vec<usize>,
}

impl<'a> Layout<'a> {
  /// Builds the direct-storage dependency graph for all type definitions.
  pub fn new(ast: &[Definition<'a>]) -> Self {
    let mut names: Vec<_> = ast //.
      .iter()
      .filter(|def| def.kind == DefinitionKind::Type)
      .flat_map(|def| [def.name, def.r#type])
      .collect();
    names.sort_unstable();
    names.dedup();

    let edges = ast
      .iter()
      .filter(|def| def.kind == DefinitionKind::Type)
      .flat_map(|def| {
        let &Definition { r#type, name, ref fields, .. } = def;
        let enum_payload = (!fields.is_empty() && r#type != name).then_some([r#type, name]);
        let direct_fields = fields.iter().filter_map(move |field| Some([name, direct_type(&field.r#type)?]));
        enum_payload.into_iter().chain(direct_fields)
      })
      .filter_map(|edge| match edge.map(|name| names.binary_search(&name)) {
        [Ok(src), Ok(dst)] => Some([src, dst]),
        _ => None,
      })
      .collect();

    let graph = Graph::from_edges(edges, names.len());
    let components = graph.strongly_connected_components();
    Self { names, components }
  }

  /// Reports whether a reference from `parent` to `target` forms a layout cycle.
  pub fn is_recursive(&self, parent: &str, target: &str) -> bool {
    let [Ok(src), Ok(dst)] = [parent, target].map(|name| self.names.binary_search(&name)) else { return false };
    let [Some(&src), Some(&dst)] = [src, dst].map(|idx| self.components.get(idx)) else { return false };
    src == dst
  }
}

/// Returns a directly stored non-primitive type name.
fn direct_type<'a>(expr: &TypeExpr<'a>) -> Option<&'a str> {
  match *expr {
    TypeExpr::Bare(name) if let None = schema::primitive(name) => Some(name),
    TypeExpr::Bare(_) | TypeExpr::Vector(_) => None,
  }
}

#[cfg(test)]
mod tests {
  use td_parser::parse;

  use super::*;

  #[test]
  fn separates_acyclic_dependencies() {
    let ast = parse("user id:int32 = User; message author:User = Message;").unwrap();
    let layout = Layout::new(&ast);

    assert!(!layout.is_recursive("message", "User"));
    assert!(!layout.is_recursive("User", "Message"));
  }

  #[test]
  fn detects_multi_type_cycle() {
    let ast = parse("a next:B = A; b next:C = B; c next:A = C;").unwrap();
    let layout = Layout::new(&ast);

    for [parent, target] in [["a", "B"], ["b", "C"], ["c", "A"]] {
      assert!(layout.is_recursive(parent, target));
    }
  }

  #[test]
  fn vector_breaks_cycle() {
    let ast = parse("folder subfolders:vector<Category> = Category;").unwrap();
    let layout = Layout::new(&ast);

    assert!(!layout.is_recursive("folder", "Category"));
  }
}
