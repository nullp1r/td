//! Zero-copy parsing for `TDLib`'s Type Language API schema.
//!
//! This crate parses the subset of TL used by `td_api.tl` into a compact,
//! borrowed syntax tree. Constructor and function names, type names, and
//! documentation remain slices of the input; only the definition and field
//! structure is allocated. The parser also carries `//@description`,
//! `//@class`, and field documentation into the AST for code generation.
//!
//! The representation is intentionally specialized for generating the JSON API:
//! constructor IDs and generic declarations are accepted but discarded, while
//! vectors and direct named references are retained because they determine the
//! generated Rust layout.
//!
//! ```
//! use td_parser::{DefinitionKind, TypeExpr, parse};
//!
//! let schema = r#"
//!   //@description A user
//!   //@id User identifier
//!   user id:int64 = User;
//!   ---functions---
//!   //@description Returns a user
//!   //@id User identifier
//!   getUser id:int64 = User;
//! "#;
//!
//! let definitions = parse(schema).expect("valid TD API schema");
//! assert_eq!(definitions.len(), 2);
//! assert_eq!(definitions[0].kind, DefinitionKind::Type);
//! assert_eq!(definitions[0].comb.fields[0].r#type, TypeExpr::Bare("int64"));
//! assert_eq!(definitions[1].kind, DefinitionKind::Function);
//! ```

pub use ast::*;
pub use error::*;
pub use parser::*;

mod ast;
mod cursor;
mod error;
mod parser;
