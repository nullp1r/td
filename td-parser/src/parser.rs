use std::iter;

use crate::ast::{Combinator, Definition, DefinitionKind, Field, TypeExpr};
use crate::cursor::Cursor;
use crate::error::Error;

impl<'a> Cursor<'a> {
  fn skip_params(&mut self) -> Result<(), Error<'a>> {
    loop {
      self.skip_ws();
      if self.maybe("#") {
        self.hex();
        self.skip_ws();
        self.maybe_balanced(['[', ']'])?;
      } else if !self.maybe_balanced(['{', '}'])? && !self.maybe("?") {
        return Ok(());
      }
    }
  }

  fn field(&mut self, doc: &'a str) -> Result<Field<'a>, Error<'a>> {
    let name = self.ident().ok_or(Error::UnexpectedInput(self.rest))?;
    self.skip_ws();
    self.expect(":")?;
    let type_expr = self.type_expr()?;
    let desc = doc_tags(doc).find(|&[n, _]| n == name).map(|[_, d]| d);
    let is_optional = desc.is_some_and(is_optional);
    Ok(Field { name, type_expr, is_optional, desc })
  }

  fn type_expr(&mut self) -> Result<TypeExpr<'a>, Error<'a>> {
    self.skip_ws();
    if self.maybe("vector") {
      self.skip_ws();
      if self.maybe("<") {
        let inner = self.type_expr()?;
        self.skip_ws();
        self.expect(">")?;
        return Ok(TypeExpr::Vector(Box::new(inner)));
      }
      return Ok(TypeExpr::Bare("vector"));
    }
    if self.maybe("?") {
      return Ok(TypeExpr::Bare("?"));
    }
    if self.maybe("#") {
      return Ok(TypeExpr::Bare("#"));
    }
    let Some(name) = self.ident() else {
      return Err(Error::ExpectedTypeExpr);
    };
    self.skip_ws();
    self.maybe_balanced(['<', '>'])?;
    Ok(TypeExpr::Bare(name))
  }

  fn definition(&mut self, kind: &mut DefinitionKind) -> Result<Option<Definition<'a>>, Error<'a>> {
    loop {
      let doc = self.comments();

      if self.rest.is_empty() {
        return Ok(None);
      }

      if self.maybe("---") {
        let (marker, tail) = self.rest.split_once('\n').unwrap_or((self.rest, ""));
        self.rest = tail;
        *kind = match marker.trim_matches(['-', ' ']) {
          "types" => DefinitionKind::Type,
          "functions" => DefinitionKind::Function,
          _ => return Err(Error::ExpectedDefinitionKind),
        };
        continue;
      }

      let Some(name) = self.ident() else {
        return Err(Error::UnexpectedInput(self.rest));
      };
      self.skip_params()?;

      let mut fields = Vec::new();
      while {
        self.skip_ws();
        !self.maybe("=")
      } {
        fields.push(self.field(doc)?);
      }

      self.skip_ws();
      let Some(category) = self.ident() else {
        return Err(Error::ExpectedCategory);
      };
      self.skip_ws();
      while let Some(_) = self.ident() {
        self.skip_ws();
      }
      self.expect(";")?;

      let [class, desc] = doc_class_and_desc(doc);
      let comb = Combinator { name, fields, category, desc, class };
      return Ok(Some(Definition { kind: *kind, comb }));
    }
  }
}

fn is_optional(desc: &str) -> bool {
  desc.contains("may be null") || desc.contains("pass null")
}

fn doc_class_and_desc(doc: &str) -> [Option<&str>; 2] {
  doc_tags(doc).fold([None; 2], |[class, desc], [k, v]| match k {
    "class" if let Some((_, v)) = v.split_once(" @description ") => [Some(v), desc],
    "description" => [class, Some(v)],
    _ => [class, desc],
  })
}

fn doc_tags(doc: &str) -> impl Iterator<Item = [&str; 2]> {
  doc.split("//@").skip(1).filter_map(|part| {
    let (k, v) = part.trim().split_once(char::is_whitespace)?;
    Some([k, v])
  })
}

pub fn parse(input: &str) -> Result<Vec<Definition<'_>>, Error<'_>> {
  let mut cur = Cursor::new(input);
  let mut kind = DefinitionKind::Type;
  iter::from_fn(|| cur.definition(&mut kind).transpose()).collect()
}
