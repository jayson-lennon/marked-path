use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

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
/// use std::path::PathBuf;
/// use marked_path::{MarkedPath, Absolute, Relative};
///
/// // Create an absolute path (validated at construction)
/// let abs = MarkedPath::<Absolute>::new(PathBuf::from("/home/user"))?;
///
/// // Create a relative path
/// let rel = MarkedPath::<Relative>::new(PathBuf::from("documents/file.txt"))?;
///
/// // You can push relative paths onto absolute paths
/// let mut abs = MarkedPath::<Absolute>::new(PathBuf::from("/home"))?;
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
