// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 3 of the License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
// 
// You should have received a copy of the GNU Lesser General Public License
// along with this program; if not, see <https://opensource.org/license/lgpl-3-0>.

//! Type-safe path wrappers with compile-time absolute/relative guarantees.
//!
//! This crate provides [`MarkedPathBuf<M>`], an owned wrapper around
//! [`std::path::PathBuf`], and [`MarkedPath<'a, M>`], a borrowed wrapper around
//! [`std::path::Path`]. Both use a zero-sized marker type to enforce at compile
//! time whether a path is [`Absolute`] or [`Relative`]. It also provides
//! [`CanonicalPath`], which guarantees a fully resolved, existing filesystem
//! path.
//!
//! Errors are reported as [`PathError`].
//!
//! # Example
//!
//! ```
//! use marked_path::{Absolute, MarkedPathBuf, Relative};
//!
//! let abs = MarkedPathBuf::<Absolute>::new("/home/user")?;
//! let rel = MarkedPathBuf::<Relative>::new("documents/file.txt")?;
//!
//! # Ok::<(), marked_path::PathError>(())
//! ```

mod absolute;
mod canonical;
mod marked_path;
mod relative;

pub use absolute::Absolute;
pub use canonical::CanonicalPath;
pub use marked_path::{MarkedAncestors, MarkedPath, MarkedPathBuf, PathError};
pub use relative::Relative;
