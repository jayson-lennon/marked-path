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

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::marker::PhantomData;
use std::path::{Ancestors, Path, PathBuf};

use wherror::Error;

use crate::path_access::{MarkedPathAccess, PathWrapper};

/// Error type for path operations.
///
/// This error is returned when a path operation fails, such as attempting to
/// create a `MarkedPathBuf<Absolute>` from a relative path, or vice versa.
#[derive(Debug, Error)]
pub enum PathError {
    /// The path was expected to be absolute but was relative.
    #[error("path is not absolute")]
    NotAbsolute,

    /// The path was expected to be relative but was absolute.
    #[error("path is not relative")]
    NotRelative,

    /// The path is not in canonical form (contains `.`, `..`, or is a symlink).
    #[error("path is not in canonical form")]
    NotCanonical,

    /// An I/O error occurred during path operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// An owned, type-safe path wrapper with a zero-sized marker.
///
/// See [`MarkedPath`] for the borrowed equivalent.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkedPathBuf<M> {
    pub(crate) path: PathBuf,
    pub(crate) _marker: PhantomData<M>,
}

impl<M> Clone for MarkedPathBuf<M> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _marker: PhantomData,
        }
    }
}

impl<M> fmt::Display for MarkedPathBuf<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<M> AsRef<Path> for MarkedPathBuf<M> {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl<M> From<MarkedPathBuf<M>> for PathBuf {
    fn from(value: MarkedPathBuf<M>) -> Self {
        value.path
    }
}

impl<M> PathWrapper for MarkedPathBuf<M> {
    type Owned = MarkedPathBuf<M>;
    type Borrowed<'a>
        = MarkedPath<'a, M>
    where
        M: 'a;
    type Ancestors<'a>
        = MarkedAncestors<'a, M>
    where
        M: 'a;

    fn wrap_buf(path: PathBuf) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path,
            _marker: PhantomData,
        }
    }

    fn wrap_ref(path: &Path) -> MarkedPath<'_, M> {
        MarkedPath {
            path,
            _marker: PhantomData,
        }
    }
}

impl<M> MarkedPathAccess for MarkedPathBuf<M> {
    fn as_path(&self) -> &Path {
        &self.path
    }

    fn ancestors(&self) -> MarkedAncestors<'_, M> {
        MarkedAncestors {
            inner: self.path.ancestors(),
            _marker: PhantomData,
        }
    }
}

impl<M> MarkedPathBuf<M> {
    /// Consumes this `MarkedPathBuf` and returns the underlying [`PathBuf`].
    pub fn into_inner(self) -> PathBuf {
        self.path
    }

    /// Consumes this `MarkedPathBuf` and returns the underlying [`OsString`].
    ///
    /// See [`PathBuf::into_os_string`](std::path::PathBuf::into_os_string).
    pub fn into_os_string(self) -> OsString {
        self.path.into_os_string()
    }

    /// Returns a borrowed [`MarkedPath`] view of this path.
    pub fn as_marked_path(&self) -> MarkedPath<'_, M> {
        MarkedPath {
            path: &self.path,
            _marker: PhantomData,
        }
    }

    /// Truncates this path to its parent.
    ///
    /// See [`PathBuf::pop`](std::path::PathBuf::pop).
    pub fn pop(&mut self) -> bool {
        self.path.pop()
    }

    /// Updates [`self.extension`](Path::extension) to the given extension.
    ///
    /// See [`PathBuf::set_extension`](std::path::PathBuf::set_extension).
    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        self.path.set_extension(extension)
    }

    /// Appends to [`self.extension`](Path::extension).
    ///
    /// See [`PathBuf::add_extension`](std::path::PathBuf::add_extension).
    pub fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        self.path.add_extension(extension)
    }
}

/// A borrowed, type-safe path reference with a zero-sized marker.
///
/// See [`MarkedPathBuf`] for the owned equivalent.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkedPath<'a, M> {
    pub(crate) path: &'a Path,
    pub(crate) _marker: PhantomData<M>,
}

impl<M> fmt::Display for MarkedPath<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<M> AsRef<Path> for MarkedPath<'_, M> {
    fn as_ref(&self) -> &Path {
        self.path
    }
}

impl<M> PathWrapper for MarkedPath<'_, M> {
    type Owned = MarkedPathBuf<M>;
    type Borrowed<'b>
        = MarkedPath<'b, M>
    where
        Self: 'b;
    type Ancestors<'b>
        = MarkedAncestors<'b, M>
    where
        Self: 'b;

    fn wrap_buf(path: PathBuf) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path,
            _marker: PhantomData,
        }
    }

    fn wrap_ref(path: &Path) -> MarkedPath<'_, M> {
        MarkedPath {
            path,
            _marker: PhantomData,
        }
    }
}

impl<'a, M> MarkedPathAccess for MarkedPath<'a, M> {
    fn as_path(&self) -> &'a Path {
        self.path
    }

    fn ancestors(&self) -> MarkedAncestors<'a, M> {
        MarkedAncestors {
            inner: self.path.ancestors(),
            _marker: PhantomData,
        }
    }
}

impl<M> MarkedPath<'_, M> {
    /// Returns an owned [`MarkedPathBuf`] cloning the path data.
    pub fn to_owned(&self) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.to_path_buf(),
            _marker: PhantomData,
        }
    }
}

/// An iterator over [`MarkedPath`] ancestors.
///
/// See [`Path::ancestors`](std::path::Path::ancestors).
pub struct MarkedAncestors<'a, M> {
    inner: Ancestors<'a>,
    _marker: PhantomData<M>,
}

impl<'a, M> Iterator for MarkedAncestors<'a, M> {
    type Item = MarkedPath<'a, M>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|p| MarkedPath {
            path: p,
            _marker: PhantomData,
        })
    }
}

impl<'a, M> From<std::path::Ancestors<'a>> for MarkedAncestors<'a, M> {
    fn from(inner: std::path::Ancestors<'a>) -> Self {
        MarkedAncestors {
            inner,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::absolute::Absolute;
    use crate::relative::Relative;
    use rstest::rstest;

    #[rstest]
    fn marked_path_display() {
        let path =
            MarkedPathBuf::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(format!("{path}"), "some/relative/path");
    }

    #[rstest]
    fn marked_path_to_path_buf() {
        let path =
            MarkedPathBuf::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(
            path.to_path_buf(),
            std::path::PathBuf::from("some/relative/path")
        );
    }

    #[rstest]
    fn marked_path_into_inner() {
        let path =
            MarkedPathBuf::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(
            path.into_inner(),
            std::path::PathBuf::from("some/relative/path")
        );
    }

    #[rstest]
    fn marked_path_parent_on_relative() {
        let buf = MarkedPathBuf::<Relative>::new("a/b/c").unwrap();
        let parent = buf.parent().unwrap();
        assert_eq!(parent.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn marked_path_parent_on_absolute() {
        let buf = MarkedPathBuf::<Absolute>::new("/a/b/c").unwrap();
        let parent = buf.parent().unwrap();
        assert_eq!(parent.as_path(), std::path::Path::new("/a/b"));
    }

    #[rstest]
    fn marked_path_parent_on_root() {
        let buf = MarkedPathBuf::<Absolute>::new("/").unwrap();
        assert!(buf.parent().is_none());
    }

    #[rstest]
    fn marked_path_parent_on_single_component() {
        let buf = MarkedPathBuf::<Relative>::new("foo").unwrap();
        let parent = buf.parent().unwrap();
        assert_eq!(parent.as_path(), std::path::Path::new(""));
    }

    #[rstest]
    fn marked_path_parent_on_empty() {
        let buf = MarkedPathBuf::<Relative>::new("").unwrap();
        assert!(buf.parent().is_none());
    }

    #[rstest]
    fn marked_ancestors_yields_typed_paths() {
        let buf = MarkedPathBuf::<Relative>::new("a/b/c").unwrap();
        let ancestors: Vec<_> = buf.ancestors().collect();
        assert_eq!(ancestors.len(), 4);
        assert_eq!(ancestors[0].as_path(), std::path::Path::new("a/b/c"));
        assert_eq!(ancestors[1].as_path(), std::path::Path::new("a/b"));
        assert_eq!(ancestors[2].as_path(), std::path::Path::new("a"));
        assert_eq!(ancestors[3].as_path(), std::path::Path::new(""));
    }

    #[rstest]
    fn deref_from_buf_to_path() {
        let buf: MarkedPathBuf<Relative> = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let borrowed: MarkedPath<Relative> = MarkedPath {
            path: buf.as_path(),
            _marker: PhantomData,
        };
        assert_eq!(borrowed.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn to_owned_from_borrowed() {
        let buf = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let borrowed: MarkedPath<Relative> = MarkedPath {
            path: buf.as_path(),
            _marker: PhantomData,
        };
        let owned = borrowed.to_owned();
        assert_eq!(owned.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn marked_path_into_pathbuf() {
        let path = MarkedPathBuf::<Relative>::new("some/relative/path").unwrap();
        let pathbuf: PathBuf = path.into();
        assert_eq!(pathbuf, PathBuf::from("some/relative/path"));
    }
}
