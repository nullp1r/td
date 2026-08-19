#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DefinitionKind {
  Type,
  Function,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Definition<'a> {
  pub kind: DefinitionKind,
  pub comb: Combinator<'a>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Combinator<'a> {
  pub r#type: &'a str,
  pub name: &'a str,
  pub fields: Vec<Field<'a>>,
  pub desc: Option<&'a str>,
  pub meta: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Field<'a> {
  pub is_optional: bool,
  pub name: &'a str,
  pub r#type: TypeExpr<'a>,
  pub desc: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeExpr<'a> {
  Bare(&'a str),
  Vector(Box<TypeExpr<'a>>),
}
