use td_parser::{Combinator, Definition, DefinitionKind};

use crate::scc::SccMap;
use crate::util;
pub struct Context<'a> {
  scc: SccMap<'a>,
  enums: Vec<&'a str>,
  types: Vec<&'a Combinator<'a>>,
  fns: Vec<&'a Combinator<'a>>,
}

impl<'a> Context<'a> {
  pub fn new(ast: &'a [Definition<'a>]) -> Self {
    let scc = SccMap::from_ast(ast);

    let mut enums: Vec<_> = ast.iter().map(|d| d.comb.category).collect();
    enums.sort_unstable();
    enums.dedup();

    let (mut types, mut fns) = (Vec::default(), Vec::default());
    for d in ast {
      match d.kind {
        DefinitionKind::Type => types.push(&d.comb),
        DefinitionKind::Function => fns.push(&d.comb),
      }
    }

    types.sort_unstable_by_key(|c| [c.category, c.name]);

    Self { scc, enums, types, fns }
  }

  pub fn groups(&self) -> impl Iterator<Item = (&'a str, &[&'a Combinator<'a>])> {
    let chunks = self.types.chunk_by(|a, b| a.category == b.category);
    chunks.filter_map(|group| {
      let &[comb, ..] = group else { return None };
      let None = util::to_native(comb.category) else { return None };
      Some((comb.category, group))
    })
  }

  pub fn fns(&self) -> &[&'a Combinator<'a>] {
    &self.fns
  }

  pub fn is_enum(&self, name: &str) -> bool {
    self.enums.binary_search(&name).is_ok()
  }

  pub fn in_same_scc(&self, names: [&str; 2]) -> bool {
    self.scc.in_same_scc(names)
  }
}
