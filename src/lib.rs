//! Type-safe path wrappers with compile-time absolute/relative guarantees.
//!
//! This crate provides [`MarkedPath<M>`], a wrapper around [`std::path::PathBuf`]
//! that uses a zero-sized marker type to enforce at compile time whether a path
//! is [`Absolute`] or [`Relative`]. It also provides [`CanonicalPath`], which
//! guarantees a fully resolved, existing filesystem path.
//!
//! Errors are reported as [`PathError`].
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//! use marked_path::{Absolute, MarkedPath, Relative};
//!
//! let abs = MarkedPath::<Absolute>::new(PathBuf::from("/home/user"))?;
//! let rel = MarkedPath::<Relative>::new(PathBuf::from("documents/file.txt"))?;
//!
//! # Ok::<(), marked_path::PathError>(())
//! ```

mod absolute;
mod canonical;
mod marked_path;
mod relative;

pub use absolute::Absolute;
pub use canonical::CanonicalPath;
pub use marked_path::{MarkedPath, PathError};
pub use relative::Relative;
