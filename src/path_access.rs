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
use std::ffi::OsStr;
use std::fs::{Metadata, ReadDir};
use std::path::{Components, Path, PathBuf};

mod sealed {
    use std::path::{Path, PathBuf};

    pub trait PathWrapper {
        type Owned;
        type Borrowed<'a>
        where
            Self: 'a;
        type Ancestors<'a>: From<std::path::Ancestors<'a>>
        where
            Self: 'a;

        fn wrap_buf(path: PathBuf) -> Self::Owned;
        fn wrap_ref<'a>(path: &'a Path) -> Self::Borrowed<'a>;
    }
}

pub(crate) use sealed::PathWrapper;

/// A trait providing common path query operations for marked path types.
///
/// This trait is implemented by [`MarkedPathBuf`](crate::MarkedPathBuf),
/// [`MarkedPath`](crate::MarkedPath), and [`CanonicalPath`](crate::CanonicalPath),
/// giving all three types a shared set of read-only path methods with default
/// implementations that delegate to the underlying [`Path`].
///
/// # Sealed
///
/// This trait cannot be implemented outside of this crate.
pub trait MarkedPathAccess: PathWrapper {
    /// Returns a reference to the underlying [`Path`].
    fn as_path(&self) -> &Path;

    /// Returns the final component of this path.
    ///
    /// See [`Path::file_name`](std::path::Path::file_name).
    fn file_name(&self) -> Option<&OsStr> {
        self.as_path().file_name()
    }

    /// Returns the file stem (portion before the final `.`).
    ///
    /// See [`Path::file_stem`](std::path::Path::file_stem).
    fn file_stem(&self) -> Option<&OsStr> {
        self.as_path().file_stem()
    }

    /// Returns the extension (portion after the final `.`).
    ///
    /// See [`Path::extension`](std::path::Path::extension).
    fn extension(&self) -> Option<&OsStr> {
        self.as_path().extension()
    }

    /// Returns the file prefix (portion before the first `.` in the file name).
    ///
    /// See [`Path::file_prefix`](std::path::Path::file_prefix).
    fn file_prefix(&self) -> Option<&OsStr> {
        self.as_path().file_prefix()
    }

    /// Produces an iterator over this path and its ancestors.
    ///
    /// The default implementation returns [`std::path::Ancestors`].
    /// Implementations that carry a marker type (e.g. `MarkedPathBuf<M>`,
    /// `MarkedPath<'_, M>`) override this to return a typed iterator.
    ///
    /// See [`Path::ancestors`](std::path::Path::ancestors).
    fn ancestors(&self) -> Self::Ancestors<'_> {
        self.as_path().ancestors().into()
    }

    /// Produces an iterator over the components of this path.
    ///
    /// See [`Path::components`](std::path::Path::components).
    fn components(&self) -> Components<'_> {
        self.as_path().components()
    }

    /// Returns `true` if this path has a root component.
    ///
    /// See [`Path::has_root`](std::path::Path::has_root).
    fn has_root(&self) -> bool {
        self.as_path().has_root()
    }

    /// Returns `true` if this path is empty.
    ///
    /// See [`Path::is_empty`](std::path::Path::is_empty).
    fn is_empty(&self) -> bool {
        self.as_path().as_os_str().is_empty()
    }

    /// Returns `true` if this path starts with the given path.
    ///
    /// See [`Path::starts_with`](std::path::Path::starts_with).
    fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        self.as_path().starts_with(base)
    }

    /// Returns `true` if this path ends with the given path.
    ///
    /// See [`Path::ends_with`](std::path::Path::ends_with).
    fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        self.as_path().ends_with(child)
    }

    /// Returns `true` if this path exists on the filesystem.
    ///
    /// See [`Path::exists`](std::path::Path::exists).
    fn exists(&self) -> bool {
        self.as_path().exists()
    }

    /// Returns `Ok(true)` if the path exists, `Ok(false)` if not, or an error.
    ///
    /// See [`Path::try_exists`](std::path::Path::try_exists).
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying filesystem call.
    fn try_exists(&self) -> Result<bool, std::io::Error> {
        self.as_path().try_exists()
    }

    /// Returns `true` if this path exists and is a regular file.
    ///
    /// See [`Path::is_file`](std::path::Path::is_file).
    fn is_file(&self) -> bool {
        self.as_path().is_file()
    }

    /// Returns `true` if this path exists and is a directory.
    ///
    /// See [`Path::is_dir`](std::path::Path::is_dir).
    fn is_dir(&self) -> bool {
        self.as_path().is_dir()
    }

    /// Returns `true` if this path exists and is a symbolic link.
    ///
    /// See [`Path::is_symlink`](std::path::Path::is_symlink).
    fn is_symlink(&self) -> bool {
        self.as_path().is_symlink()
    }

    /// Reads the metadata for the path referenced by this path.
    ///
    /// See [`Path::metadata`](std::path::Path::metadata).
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying filesystem call.
    fn metadata(&self) -> Result<Metadata, std::io::Error> {
        self.as_path().metadata()
    }

    /// Reads the symbolic link metadata for the path referenced by this path.
    ///
    /// See [`Path::symlink_metadata`](std::path::Path::symlink_metadata).
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying filesystem call.
    fn symlink_metadata(&self) -> Result<Metadata, std::io::Error> {
        self.as_path().symlink_metadata()
    }

    /// Returns an iterator over the entries in this directory path.
    ///
    /// See [`Path::read_dir`](std::path::Path::read_dir).
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying filesystem call.
    fn read_dir(&self) -> Result<ReadDir, std::io::Error> {
        self.as_path().read_dir()
    }

    /// Returns this path as a `&str` if it is valid UTF-8.
    ///
    /// See [`Path::to_str`](std::path::Path::to_str).
    fn to_str(&self) -> Option<&str> {
        self.as_path().to_str()
    }

    /// Converts this path to a `Cow<str>`, replacing invalid UTF-8 with
    /// replacement characters.
    ///
    /// See [`Path::to_string_lossy`](std::path::Path::to_string_lossy).
    fn to_string_lossy(&self) -> Cow<'_, str> {
        self.as_path().to_string_lossy()
    }

    /// Returns the underlying [`OsStr`] slice.
    ///
    /// See [`Path::as_os_str`](std::path::Path::as_os_str).
    fn as_os_str(&self) -> &OsStr {
        self.as_path().as_os_str()
    }

    /// Returns a clone of the underlying [`PathBuf`].
    ///
    /// See [`Path::to_path_buf`](std::path::Path::to_path_buf).
    fn to_path_buf(&self) -> PathBuf {
        self.as_path().to_path_buf()
    }

    /// Returns the parent path, if any.
    ///
    /// See [`Path::parent`](std::path::Path::parent).
    fn parent(&self) -> Option<Self::Borrowed<'_>> {
        self.as_path().parent().map(|p| Self::wrap_ref(p))
    }

    /// Returns a new owned path with the extension replaced.
    ///
    /// See [`Path::with_extension`](std::path::Path::with_extension).
    #[must_use = "returning a new path without using it is likely a mistake"]
    fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> Self::Owned {
        Self::wrap_buf(self.as_path().with_extension(extension))
    }

    /// Returns a new owned path with the extension appended.
    ///
    /// See [`Path::with_added_extension`](std::path::Path::with_added_extension).
    #[must_use = "returning a new path without using it is likely a mistake"]
    fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> Self::Owned {
        Self::wrap_buf(self.as_path().with_added_extension(extension))
    }
}
