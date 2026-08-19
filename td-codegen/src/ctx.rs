//! Code generation context for `TDLib` schema AST definitions.
//!
//! Categorizes definitions into types, enums, and functions while providing
//! SCC recursive cycle lookups.

use td_parser::{Combinator, Definition, DefinitionKind};

use crate::scc::SccMap;
use crate::util;

/// Code generation context containing categorized definitions and SCC metadata.
pub struct Context<'a> {
  /// SCC cycle detection map for checking mutually recursive type dependencies.
  scc: SccMap<'a>,
  /// Unique list of enum category names.
  enums: Vec<&'a str>,
  /// Sorted type combinator definitions.
  types: Vec<&'a Combinator<'a>>,
  /// Function combinator definitions.
  fns: Vec<&'a Combinator<'a>>,
}

impl<'a> Context<'a> {
  /// Builds the generation context from parsed AST definitions.
  pub fn new(ast: &'a [Definition<'a>]) -> Self {
    let (mut enums, mut types, mut fns): (Vec<_>, Vec<_>, Vec<_>) = Default::default();

    for d in ast {
      let target = match d.kind {
        DefinitionKind::Type => &mut types,
        DefinitionKind::Function => &mut fns,
      };
      target.push(&d.comb);
      enums.push(d.comb.category);
    }

    types.sort_unstable_by_key(|c| [c.category, c.name]);

    enums.sort_unstable();
    enums.dedup();

    let scc = SccMap::from_ast(ast);
    Self { scc, enums, types, fns }
  }

  /// Yields non-primitive type combinators grouped by category name.
  pub fn groups(&self) -> impl Iterator<Item = (&'a str, &[&'a Combinator<'a>])> {
    let groups = self.types.chunk_by(|a, b| a.category == b.category);
    groups.filter_map(|group| {
      let &[comb, ..] = group else { return None };
      let None = util::to_native(comb.category) else { return None };
      Some((comb.category, group))
    })
  }

  /// Returns all function combinator definitions.
  pub fn fns(&self) -> &[&'a Combinator<'a>] {
    &self.fns
  }

  /// Returns `true` if `name` is a known enum category.
  pub fn is_enum(&self, name: &str) -> bool {
    self.enums.binary_search(&name).is_ok()
  }

  /// Returns `true` if both type names belong to the same recursive SCC cycle.
  pub fn in_same_scc(&self, [a, b]: [&str; 2]) -> bool {
    self.scc.in_same_scc([a, b])
  }
}
