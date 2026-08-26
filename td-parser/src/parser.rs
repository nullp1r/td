//! Recursive-descent parsing for the JSON-oriented subset of TL.
//!
//! The cursor advances through the source once. Balanced generic and parameter
//! groups are skipped because their contents do not affect JSON code generation;
//! direct named references and vector nesting are retained in [`TypeExpr`].

use std::iter;

use crate::ast::{Combinator, Definition, DefinitionKind, Field, TypeExpr};
use crate::cursor::Cursor;
use crate::error::Error;

impl<'a> Cursor<'a> {
  /// Skips constructor IDs and TL generic/parameter declarations after a name.
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

  /// Parses one field and attaches its same-named documentation tag.
  fn field(&mut self, doc: &'a str) -> Result<Field<'a>, Error<'a>> {
    let name = self.ident().ok_or(Error::UnexpectedInput(self.rest))?;
    self.skip_ws();
    self.expect(":")?;
    let r#type = self.type_expr()?;
    let desc = doc_tags(doc).find(|&[n, _]| n == name).map(|[_, d]| d);
    let is_optional = desc.is_some_and(is_optional);
    Ok(Field { is_optional, name, r#type, desc })
  }

  /// Parses the storage-relevant part of a TL type expression.
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

  /// Parses the next definition, updating the active section at marker lines.
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
      let Some(r#type) = self.ident() else {
        return Err(Error::ExpectedEnum);
      };
      self.skip_ws();
      while let Some(_) = self.ident() {
        self.skip_ws();
      }
      self.expect(";")?;

      let [desc, meta] = desc_and_meta_desc(doc);
      let comb = Combinator { r#type, name, fields, desc, meta };
      return Ok(Some(Definition { kind: *kind, comb }));
    }
  }
}

fn is_optional(desc: &str) -> bool {
  desc.contains("may be null") || desc.contains("pass null")
}

/// Separates constructor documentation from category-level `@class` metadata.
fn desc_and_meta_desc(doc: &str) -> [Option<&str>; 2] {
  let (mut is_meta, mut meta, mut desc) = Default::default();
  for [key, value] in doc_tags(doc) {
    match key {
      "class" => is_meta = true,
      "description" if is_meta => (is_meta, meta) = (false, Some(value)),
      "description" => desc = Some(value),
      _ => {}
    }
  }
  [desc, meta]
}

/// Iterates both line-leading and compact inline `@name value` documentation tags.
///
/// Upstream uses both `//@description ...\n//@field ...` and the compact
/// `//@description ... @field ...` form. A space before `@` is the delimiter for
/// an inline tag; continuation lines beginning `//-` remain part of its value.
fn doc_tags(doc: &str) -> impl Iterator<Item = [&str; 2]> {
  doc.split("//@").skip(1).flat_map(|part| part.split(" @")).filter_map(|part| {
    let (key, value) = part.trim_ascii().split_once(char::is_whitespace)?;
    Some([key, value])
  })
}

/// Parses a `TDLib` API schema into definitions borrowing from `input`.
///
/// The parser recognizes type/function section markers, constructors, functions,
/// direct and vector field types, and the documentation syntax used by
/// `td_api.tl`. Constructor IDs and generic declarations are validated only for
/// balanced delimiters, then omitted from the returned JSON-oriented AST.
/// Definitions before the first section marker are treated as types.
///
/// # Errors
///
/// Returns the first malformed delimiter, missing token, invalid section marker,
/// or unexpected input suffix.
pub fn parse(input: &str) -> Result<Vec<Definition<'_>>, Error<'_>> {
  let mut cur = Cursor::new(input);
  let mut kind = DefinitionKind::Type;
  iter::from_fn(|| cur.definition(&mut kind).transpose()).collect()
}
