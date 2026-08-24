//! Sorted indexes and layout analysis shared by source-formatting helpers.

use td_parser::{Combinator, Definition, DefinitionKind};

use crate::scc::SccMap;
use crate::util;

/// Precomputed schema views used during one generation pass.
///
/// Constructor definitions are sorted into contiguous result-type groups. The
/// separate sorted name table makes enum recognition a binary search, while the
/// SCC map answers whether a direct reference needs layout indirection.
pub struct Context<'a> {
  /// Recursive-layout components for direct type references.
  scc: SccMap<'a>,
  /// Sorted, deduplicated TL result-type names.
  enums: Vec<&'a str>,
  /// Type constructors sorted by `[result type, constructor name]`.
  types: Vec<&'a Combinator<'a>>,
  /// Functions retained in schema order.
  fns: Vec<&'a Combinator<'a>>,
}

impl<'a> Context<'a> {
  /// Builds deterministic lookup tables for `ast`.
  pub fn new(ast: &'a [Definition<'a>]) -> Self {
    let (mut enums, mut types, mut fns): (Vec<_>, Vec<_>, Vec<_>) = Default::default();

    for d in ast {
      let combs = match d.kind {
        DefinitionKind::Type => &mut types,
        DefinitionKind::Function => &mut fns,
      };
      combs.push(&d.comb);
      enums.push(d.comb.r#type);
    }

    types.sort_unstable_by_key(|c| [c.r#type, c.name]);

    enums.sort_unstable();
    enums.dedup();

    let scc = SccMap::from_ast(ast);
    Self { scc, enums, types, fns }
  }

  /// Iterates non-native result types and their contiguous constructor groups.
  pub fn groups(&self) -> impl Iterator<Item = (&'a str, &[&'a Combinator<'a>])> {
    let groups = self.types.chunk_by(|a, b| a.r#type == b.r#type);
    groups.filter_map(|group| {
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

  /// Reports whether directly embedding `a` and `b` would close a layout cycle.
  pub fn in_same_scc(&self, [a, b]: [&str; 2]) -> bool {
    self.scc.in_same_scc([a, b])
  }
}
