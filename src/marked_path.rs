use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{Metadata, ReadDir};
use std::hash::Hash;
use std::marker::PhantomData;
use std::path::{Ancestors, Components, Path, PathBuf};

use wherror::Error;

/// Error type for path operations.
///
/// This error is returned when a path operation fails, such as attempting to
/// create a `MarkedPath<Absolute>` from a relative path, or vice versa.
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

/// A type-safe path wrapper with an absolute/relative marker.
///
/// This struct provides compile-time guarantees about whether a path is
/// absolute or relative through its generic parameter `M`. The marker type
/// ensures that absolute and relative paths cannot be accidentally mixed.
///
/// # Type Parameters
///
/// * `M` - A marker type indicating the path's nature:
///   - [`Absolute`](crate::Absolute): The path is guaranteed to be absolute
///   - [`Relative`](crate::Relative): The path is guaranteed to be relative
///
/// # Example
///
/// ```
/// use marked_path::{MarkedPath, Absolute, Relative};
///
/// // Create an absolute path (validated at construction)
/// let abs = MarkedPath::<Absolute>::new("/home/user")?;
///
/// // Create a relative path
/// let rel = MarkedPath::<Relative>::new("documents/file.txt")?;
///
/// // You can push relative paths onto absolute paths
/// let mut abs = MarkedPath::<Absolute>::new("/home")?;
/// abs.push(&rel);
/// # Ok::<(), marked_path::PathError>(())
/// ```
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkedPath<M> {
    pub(crate) path: PathBuf,
    pub(crate) _marker: PhantomData<M>,
}

impl<M> Clone for MarkedPath<M> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _marker: PhantomData,
        }
    }
}

impl<M> fmt::Display for MarkedPath<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<M> AsRef<Path> for MarkedPath<M> {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl<M> MarkedPath<M> {
    /// Returns a reference to the underlying [`Path`].
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Returns a clone of the underlying [`PathBuf`].
    pub fn to_path_buf(&self) -> PathBuf {
        self.path.clone()
    }

    /// Consumes this `MarkedPath` and returns the underlying [`PathBuf`].
    pub fn into_inner(self) -> PathBuf {
        self.path
    }

    /// Truncates this path to its parent.
    ///
    /// See [`PathBuf::pop`](std::path::PathBuf::pop).
    pub fn pop(&mut self) -> bool {
        self.path.pop()
    }

    /// Updates [`self.file_name`](Path::file_name) to the given file name.
    ///
    /// See [`PathBuf::set_file_name`](std::path::PathBuf::set_file_name).
    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S) {
        self.path.set_file_name(file_name);
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
    pub fn ancestors(&self) -> Ancestors<'_> {
        self.path.ancestors()
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

    /// Consumes this `MarkedPath` and returns the underlying [`OsString`].
    ///
    /// See [`PathBuf::into_os_string`](std::path::PathBuf::into_os_string).
    pub fn into_os_string(self) -> OsString {
        self.path.into_os_string()
    }

    /// Returns a new `MarkedPath` with the extension replaced.
    ///
    /// See [`Path::with_extension`](std::path::Path::with_extension).
    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPath<M> {
        MarkedPath {
            path: self.path.with_extension(extension),
            _marker: PhantomData,
        }
    }

    /// Returns a new `MarkedPath` with the extension appended.
    ///
    /// See [`Path::with_added_extension`](std::path::Path::with_added_extension).
    pub fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPath<M> {
        MarkedPath {
            path: self.path.with_added_extension(extension),
            _marker: PhantomData,
        }
    }

    /// Returns a new `MarkedPath` with the file name replaced.
    ///
    /// See [`Path::with_file_name`](std::path::Path::with_file_name).
    pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> MarkedPath<M> {
        MarkedPath {
            path: self.path.with_file_name(file_name),
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relative::Relative;
    use rstest::rstest;

    #[rstest]
    fn marked_path_display() {
        let path =
            MarkedPath::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(format!("{}", path), "some/relative/path");
    }

    #[rstest]
    fn marked_path_to_path_buf() {
        let path =
            MarkedPath::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(
            path.to_path_buf(),
            std::path::PathBuf::from("some/relative/path")
        );
    }

    #[rstest]
    fn marked_path_into_inner() {
        let path =
            MarkedPath::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(
            path.into_inner(),
            std::path::PathBuf::from("some/relative/path")
        );
    }
}
