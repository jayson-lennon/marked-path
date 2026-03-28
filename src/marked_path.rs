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

    pub fn pop(&mut self) -> bool {
        self.path.pop()
    }

    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S) {
        self.path.set_file_name(file_name);
    }

    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        self.path.set_extension(extension)
    }

    pub fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        self.path.add_extension(extension)
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        self.path.file_name()
    }

    pub fn file_stem(&self) -> Option<&OsStr> {
        self.path.file_stem()
    }

    pub fn extension(&self) -> Option<&OsStr> {
        self.path.extension()
    }

    pub fn file_prefix(&self) -> Option<&OsStr> {
        self.path.file_prefix()
    }

    pub fn ancestors(&self) -> Ancestors<'_> {
        self.path.ancestors()
    }

    pub fn components(&self) -> Components<'_> {
        self.path.components()
    }

    pub fn has_root(&self) -> bool {
        self.path.has_root()
    }

    pub fn is_empty(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
        self.path.starts_with(base)
    }

    pub fn ends_with<P: AsRef<Path>>(&self, child: P) -> bool {
        self.path.ends_with(child)
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn try_exists(&self) -> Result<bool, std::io::Error> {
        self.path.try_exists()
    }

    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }

    pub fn is_dir(&self) -> bool {
        self.path.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.path.is_symlink()
    }

    pub fn metadata(&self) -> Result<Metadata, std::io::Error> {
        self.path.metadata()
    }

    pub fn symlink_metadata(&self) -> Result<Metadata, std::io::Error> {
        self.path.symlink_metadata()
    }

    pub fn read_dir(&self) -> Result<ReadDir, std::io::Error> {
        self.path.read_dir()
    }

    pub fn to_str(&self) -> Option<&str> {
        self.path.to_str()
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }

    pub fn as_os_str(&self) -> &OsStr {
        self.path.as_os_str()
    }

    pub fn into_os_string(self) -> OsString {
        self.path.into_os_string()
    }

    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPath<M> {
        MarkedPath {
            path: self.path.with_extension(extension),
            _marker: PhantomData,
        }
    }

    pub fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> MarkedPath<M> {
        MarkedPath {
            path: self.path.with_added_extension(extension),
            _marker: PhantomData,
        }
    }

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
