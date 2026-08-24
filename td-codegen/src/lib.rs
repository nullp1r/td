//! Rust source generation from a parsed `TDLib` API schema.
//!
//! This crate turns `td_parser::Definition` values into the three modules
//! consumed by `td_types`: concrete object structs in `types`, tagged result
//! enums in `enums`, and serializable requests in `fns`.
//! Generated names deliberately preserve the upstream `TDLib` spelling.
//!
//! Generation is deterministic and writes through [`std::fmt::Display`] without
//! first building a second source tree or intermediate `String`. Primitive wire
//! representations receive the required Serde adapters, and direct recursive
//! layout cycles are found with a compact dependency graph so only the necessary
//! enum references are boxed. Vectors already provide indirection and remain
//! unboxed.
//!
//! ```
//! use td_codegen::generate;
//! use td_parser::parse;
//!
//! let ast = parse("user id:int64 = User;").expect("valid schema");
//! let source = generate(&ast).to_string();
//! assert!(source.contains("pub struct user"));
//! assert!(source.contains("pub enum User"));
//! ```

pub use self::r#gen::generate;

mod ctx;
mod r#gen;
mod graph;
mod scc;
mod util;
