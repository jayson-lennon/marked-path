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

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{Metadata, ReadDir};
use std::marker::PhantomData;
use std::path::{Ancestors, Components, Path, PathBuf};

use wherror::Error;

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

impl<M> MarkedPathBuf<M> {
    /// Returns a reference to the underlying [`Path`].
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Returns a clone of the underlying [`PathBuf`].
    pub fn to_path_buf(&self) -> PathBuf {
        self.path.clone()
    }

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

    /// Returns the final component of this path.
    ///
    /// See [`Path::file_name`](std::path::Path::file_name).
    pub fn file_name(&self) -> Option<&OsStr> {
        self.path.file_name()
    }

    /// Returns the file stem (portion before the final `.`).
    ///
    /// See [`Path::file_stem`](std::path::Path::file_stem).
    pub fn file_stem(&self) -> Option<&OsStr> {
        self.path.file_stem()
    }

    /// Returns the extension (portion after the final `.`).
    ///
    /// See [`Path::extension`](std::path::Path::extension).
    pub fn extension(&self) -> Option<&OsStr> {
        self.path.extension()
    }

    /// Returns the file prefix (portion before the first `.` in the file name).
    ///
    /// See [`Path::file_prefix`](std::path::Path::file_prefix).
    pub fn file_prefix(&self) -> Option<&OsStr> {
        self.path.file_prefix()
    }

    /// Produces an iterator over this path and its ancestors.
    ///
    /// See [`Path::ancestors`](std::path::Path::ancestors).
    pub fn ancestors(&self) -> MarkedAncestors<'_, M> {
        MarkedAncestors {
            inner: self.path.ancestors(),
            _marker: PhantomData,
        }
    }

    /// Produces an iterator over the components of this path.
    ///
    /// See [`Path::components`](std::path::Path::components).
    pub fn components(&self) -> Components<'_> {
        self.path.components()
    }

    /// Returns `true` if this path has a root component.
    ///
    /// See [`Path::has_root`](std::path::Path::has_root).
    pub fn has_root(&self) -> bool {
        self.path.has_root()
    }

    /// Returns `true` if this path is empty.
    ///
    /// See [`Path::is_empty`](std::path::Path::is_empty).
    pub fn is_empty(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    /// Returns `true` if this path starts with the given path.
    ///
    /// See [`Path::starts_with`](std::path::Path::starts_with).
    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        self.path.starts_with(base)
    }

    /// Returns `true` if this path ends with the given path.
    ///
    /// See [`Path::ends_with`](std::path::Path::ends_with).
    pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        self.path.ends_with(child)
    }

    /// Returns `true` if this path exists on the filesystem.
    ///
    /// See [`Path::exists`](std::path::Path::exists).
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Returns `Ok(true)` if the path exists, `Ok(false)` if not, or an error.
    ///
    /// See [`Path::try_exists`](std::path::Path::try_exists).
    pub fn try_exists(&self) -> Result<bool, std::io::Error> {
        self.path.try_exists()
    }

    /// Returns `true` if this path exists and is a regular file.
    ///
    /// See [`Path::is_file`](std::path::Path::is_file).
    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }

    /// Returns `true` if this path exists and is a directory.
    ///
    /// See [`Path::is_dir`](std::path::Path::is_dir).
    pub fn is_dir(&self) -> bool {
        self.path.is_dir()
    }

    /// Returns `true` if this path exists and is a symbolic link.
    ///
    /// See [`Path::is_symlink`](std::path::Path::is_symlink).
    pub fn is_symlink(&self) -> bool {
        self.path.is_symlink()
    }

    /// Reads the metadata for the path referenced by this path.
    ///
    /// See [`Path::metadata`](std::path::Path::metadata).
    pub fn metadata(&self) -> Result<Metadata, std::io::Error> {
        self.path.metadata()
    }

    /// Reads the symbolic link metadata for the path referenced by this path.
    ///
    /// See [`Path::symlink_metadata`](std::path::Path::symlink_metadata).
    pub fn symlink_metadata(&self) -> Result<Metadata, std::io::Error> {
        self.path.symlink_metadata()
    }

    /// Returns an iterator over the entries in this directory path.
    ///
    /// See [`Path::read_dir`](std::path::Path::read_dir).
    pub fn read_dir(&self) -> Result<ReadDir, std::io::Error> {
        self.path.read_dir()
    }

    /// Returns this path as a `&str` if it is valid UTF-8.
    ///
    /// See [`Path::to_str`](std::path::Path::to_str).
    pub fn to_str(&self) -> Option<&str> {
        self.path.to_str()
    }

    /// Converts this path to a `Cow<str>`, replacing invalid UTF-8 with
    /// replacement characters.
    ///
    /// See [`Path::to_string_lossy`](std::path::Path::to_string_lossy).
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }

    /// Returns the underlying [`OsStr`] slice.
    ///
    /// See [`Path::as_os_str`](std::path::Path::as_os_str).
    pub fn as_os_str(&self) -> &OsStr {
        self.path.as_os_str()
    }

    /// Returns a new owned [`MarkedPathBuf`] with the extension replaced.
    ///
    /// See [`Path::with_extension`](std::path::Path::with_extension).
    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.with_extension(extension),
            _marker: PhantomData,
        }
    }

    /// Returns a new owned [`MarkedPathBuf`] with the extension appended.
    ///
    /// See [`Path::with_added_extension`](std::path::Path::with_added_extension).
    pub fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.with_added_extension(extension),
            _marker: PhantomData,
        }
    }

    /// Returns the parent path, if any.
    ///
    /// See [`Path::parent`](std::path::Path::parent).
    pub fn parent(&self) -> Option<MarkedPath<'_, M>> {
        self.path.parent().map(|p| MarkedPath {
            path: p,
            _marker: PhantomData,
        })
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

impl<'a, M> fmt::Display for MarkedPath<'a, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<'a, M> AsRef<Path> for MarkedPath<'a, M> {
    fn as_ref(&self) -> &Path {
        self.path
    }
}

impl<'a, M> MarkedPath<'a, M> {
    /// Returns a reference to the underlying [`Path`].
    pub fn as_path(&self) -> &'a Path {
        self.path
    }

    /// Returns an owned [`PathBuf`] cloning the path data.
    ///
    /// See [`Path::to_path_buf`](std::path::Path::to_path_buf).
    pub fn to_path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }

    /// Returns an owned [`MarkedPathBuf`] cloning the path data.
    pub fn to_owned(&self) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.to_path_buf(),
            _marker: PhantomData,
        }
    }

    /// Returns the final component of this path.
    ///
    /// See [`Path::file_name`](std::path::Path::file_name).
    pub fn file_name(&self) -> Option<&OsStr> {
        self.path.file_name()
    }

    /// Returns the file stem (portion before the final `.`).
    ///
    /// See [`Path::file_stem`](std::path::Path::file_stem).
    pub fn file_stem(&self) -> Option<&OsStr> {
        self.path.file_stem()
    }

    /// Returns the extension (portion after the final `.`).
    ///
    /// See [`Path::extension`](std::path::Path::extension).
    pub fn extension(&self) -> Option<&OsStr> {
        self.path.extension()
    }

    /// Returns the file prefix (portion before the first `.` in the file name).
    ///
    /// See [`Path::file_prefix`](std::path::Path::file_prefix).
    pub fn file_prefix(&self) -> Option<&OsStr> {
        self.path.file_prefix()
    }

    /// Produces an iterator over this path and its ancestors.
    ///
    /// See [`Path::ancestors`](std::path::Path::ancestors).
    pub fn ancestors(&self) -> MarkedAncestors<'a, M> {
        MarkedAncestors {
            inner: self.path.ancestors(),
            _marker: PhantomData,
        }
    }

    /// Produces an iterator over the components of this path.
    ///
    /// See [`Path::components`](std::path::Path::components).
    pub fn components(&self) -> Components<'a> {
        self.path.components()
    }

    /// Returns `true` if this path has a root component.
    ///
    /// See [`Path::has_root`](std::path::Path::has_root).
    pub fn has_root(&self) -> bool {
        self.path.has_root()
    }

    /// Returns `true` if this path is empty.
    ///
    /// See [`Path::is_empty`](std::path::Path::is_empty).
    pub fn is_empty(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    /// Returns `true` if this path starts with the given path.
    ///
    /// See [`Path::starts_with`](std::path::Path::starts_with).
    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        self.path.starts_with(base)
    }

    /// Returns `true` if this path ends with the given path.
    ///
    /// See [`Path::ends_with`](std::path::Path::ends_with).
    pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        self.path.ends_with(child)
    }

    /// Returns `true` if this path exists on the filesystem.
    ///
    /// See [`Path::exists`](std::path::Path::exists).
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Returns `Ok(true)` if the path exists, `Ok(false)` if not, or an error.
    ///
    /// See [`Path::try_exists`](std::path::Path::try_exists).
    pub fn try_exists(&self) -> Result<bool, std::io::Error> {
        self.path.try_exists()
    }

    /// Returns `true` if this path exists and is a regular file.
    ///
    /// See [`Path::is_file`](std::path::Path::is_file).
    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }

    /// Returns `true` if this path exists and is a directory.
    ///
    /// See [`Path::is_dir`](std::path::Path::is_dir).
    pub fn is_dir(&self) -> bool {
        self.path.is_dir()
    }

    /// Returns `true` if this path exists and is a symbolic link.
    ///
    /// See [`Path::is_symlink`](std::path::Path::is_symlink).
    pub fn is_symlink(&self) -> bool {
        self.path.is_symlink()
    }

    /// Reads the metadata for the path referenced by this path.
    ///
    /// See [`Path::metadata`](std::path::Path::metadata).
    pub fn metadata(&self) -> Result<Metadata, std::io::Error> {
        self.path.metadata()
    }

    /// Reads the symbolic link metadata for the path referenced by this path.
    ///
    /// See [`Path::symlink_metadata`](std::path::Path::symlink_metadata).
    pub fn symlink_metadata(&self) -> Result<Metadata, std::io::Error> {
        self.path.symlink_metadata()
    }

    /// Returns an iterator over the entries in this directory path.
    ///
    /// See [`Path::read_dir`](std::path::Path::read_dir).
    pub fn read_dir(&self) -> Result<ReadDir, std::io::Error> {
        self.path.read_dir()
    }

    /// Returns this path as a `&str` if it is valid UTF-8.
    ///
    /// See [`Path::to_str`](std::path::Path::to_str).
    pub fn to_str(&self) -> Option<&str> {
        self.path.to_str()
    }

    /// Converts this path to a `Cow<str>`, replacing invalid UTF-8 with
    /// replacement characters.
    ///
    /// See [`Path::to_string_lossy`](std::path::Path::to_string_lossy).
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }

    /// Returns the underlying [`OsStr`] slice.
    ///
    /// See [`Path::as_os_str`](std::path::Path::as_os_str).
    pub fn as_os_str(&self) -> &OsStr {
        self.path.as_os_str()
    }

    /// Returns a new owned [`MarkedPathBuf`] with the extension replaced.
    ///
    /// See [`Path::with_extension`](std::path::Path::with_extension).
    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.with_extension(extension),
            _marker: PhantomData,
        }
    }

    /// Returns a new owned [`MarkedPathBuf`] with the extension appended.
    ///
    /// See [`Path::with_added_extension`](std::path::Path::with_added_extension).
    pub fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.with_added_extension(extension),
            _marker: PhantomData,
        }
    }

    /// Returns the parent path, if any.
    ///
    /// See [`Path::parent`](std::path::Path::parent).
    pub fn parent(&self) -> Option<MarkedPath<'a, M>> {
        self.path.parent().map(|p| MarkedPath {
            path: p,
            _marker: PhantomData,
        })
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
        assert_eq!(format!("{}", path), "some/relative/path");
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
}
