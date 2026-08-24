//! Borrowed syntax tree for the `TDLib` API schema.

/// Whether a schema definition declares an object constructor or a function.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DefinitionKind {
  /// A constructor in the schema's types section.
  Type,
  /// A request in the schema's functions section.
  Function,
}

/// One classified constructor or function definition.
///
/// Every borrowed string in the contained combinator points into the original
/// schema passed to [`crate::parse`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Definition<'a> {
  /// Section in which the definition appeared.
  pub kind: DefinitionKind,
  /// Shared constructor/function syntax parsed from the definition.
  pub comb: Combinator<'a>,
}

/// The common shape of a TL object constructor or function.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Combinator<'a> {
  /// Result category for a constructor, or response type for a function.
  pub r#type: &'a str,
  /// Constructor or function name used as the JSON `@type` value.
  pub name: &'a str,
  /// Parameters in schema order.
  pub fields: Vec<Field<'a>>,
  /// Text from the definition's `@description` tag, if present.
  pub desc: Option<&'a str>,
  /// Description from a preceding `@class` declaration, if present.
  ///
  /// Class metadata describes the whole result category rather than this one
  /// constructor. The generator locates it while grouping constructors.
  pub meta: Option<&'a str>,
}

/// One named constructor field or function parameter.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Field<'a> {
  /// Whether the schema prose permits null for this field.
  ///
  /// TL does not encode JSON nullability in the field type, so this is derived
  /// from the upstream phrases “may be null” and “pass null”.
  pub is_optional: bool,
  /// Field name used on the JSON wire.
  pub name: &'a str,
  /// Parsed field type.
  pub r#type: TypeExpr<'a>,
  /// Text from the field's documentation tag, if present.
  pub desc: Option<&'a str>,
}

/// A field type relevant to generated Rust storage.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TypeExpr<'a> {
  /// A primitive, constructor, or result-category name.
  Bare(&'a str),
  /// A vector whose element type is another expression.
  Vector(Box<TypeExpr<'a>>),
}
