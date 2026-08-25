//! Sorted indexes and layout analysis shared by source-formatting helpers.

use td_parser::{Combinator, Definition, DefinitionKind};

use crate::scc::LayoutComponents;
use crate::util;

/// Deterministic schema indexes used during one generation pass.
///
/// Constructor definitions are sorted into contiguous result-type groups. The
/// separate sorted name table makes enum recognition a binary search, while the
/// layout components identify conservative indirection points.
pub struct SchemaIndex<'a> {
  /// Recursive components in the direct-storage type graph.
  layouts: LayoutComponents<'a>,
  /// Sorted, deduplicated TL result-type names.
  enums: Vec<&'a str>,
  /// Type constructors sorted by `[result type, constructor name]`.
  ctors: Vec<&'a Combinator<'a>>,
  /// Functions retained in schema order.
  fns: Vec<&'a Combinator<'a>>,
}

impl<'a> SchemaIndex<'a> {
  /// Builds deterministic lookup tables for `ast`.
  pub fn new(ast: &'a [Definition<'a>]) -> Self {
    let (mut enums, mut ctors, mut fns): (Vec<_>, Vec<_>, Vec<_>) = Default::default();

    for def in ast {
      let target = match def.kind {
        DefinitionKind::Type => &mut ctors,
        DefinitionKind::Function => &mut fns,
      };
      target.push(&def.comb);
      enums.push(def.comb.r#type);
    }

    ctors.sort_unstable_by_key(|ctor| [ctor.r#type, ctor.name]);

    enums.sort_unstable();
    enums.dedup();

    let layouts = LayoutComponents::from_ast(ast);
    Self { layouts, enums, ctors, fns }
  }

  /// Iterates non-native result types and their contiguous constructor groups.
  pub fn ctor_groups(&self) -> impl Iterator<Item = (&'a str, &[&'a Combinator<'a>])> {
    self.ctors.chunk_by(|a, b| a.r#type == b.r#type).filter_map(|group| {
      let &[comb, ..] = group else { return None };
      let None = util::to_native(comb.r#type) else { return None };
      Some((comb.r#type, group))
    })
  }

  /// Returns function combinators in their original schema order.
  pub fn fns(&self) -> &[&'a Combinator<'a>] {
    &self.fns
  }

  /// Reports whether `name` is a known TL result category.
  pub fn is_enum(&self, name: &str) -> bool {
    self.enums.binary_search(&name).is_ok()
  }

  /// Reports whether the conservative layout policy boxes this direct field.
  pub fn needs_box(&self, edge: [&str; 2]) -> bool {
    self.layouts.is_recursive_edge(edge)
  }
}
