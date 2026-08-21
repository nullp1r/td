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

  pub fn groups(&self) -> impl Iterator<Item = (&'a str, &[&'a Combinator<'a>])> {
    let groups = self.types.chunk_by(|a, b| a.r#type == b.r#type);
    groups.filter_map(|group| {
      let &[comb, ..] = group else { return None };
      let None = util::to_native(comb.r#type) else { return None };
      Some((comb.r#type, group))
    })
  }

  pub fn fns(&self) -> &[&'a Combinator<'a>] {
    &self.fns
  }

  pub fn is_enum(&self, name: &str) -> bool {
    self.enums.binary_search(&name).is_ok()
  }

  pub fn in_same_scc(&self, [a, b]: [&str; 2]) -> bool {
    self.scc.in_same_scc([a, b])
  }
}
