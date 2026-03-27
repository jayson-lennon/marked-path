use std::marker::PhantomData;
use std::path::PathBuf;
use std::str::FromStr;

use crate::canonical::CanonicalPath;
use crate::marked_path::{MarkedPath, PathError};
use crate::relative::Relative;

/// Marker type for absolute paths.
///
/// This is a phantom marker type used with [`MarkedPath`] to indicate that
/// the contained path is guaranteed to be absolute. An absolute path starts
/// from the root of the filesystem (e.g., `/path/to/file` on Unix or
/// `C:\path\to\file` on Windows).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Absolute;

impl MarkedPath<Absolute> {
    /// Creates a new `MarkedPath<Absolute>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not absolute.
    pub fn new(path: PathBuf) -> Result<Self, PathError> {
        if path.is_absolute() {
            Ok(Self {
                path,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotAbsolute)
        }
    }

    /// Canonicalizes this absolute path, returning a [`CanonicalPath`].
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path cannot be canonicalized
    /// (e.g., if it doesn't exist or there are permission issues).
    pub fn canonicalize(&self) -> Result<CanonicalPath, PathError> {
        let canonicalized = self.path.canonicalize()?;
        CanonicalPath::new(canonicalized)
    }

    /// Appends a relative path to this absolute path.
    pub fn push(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(&other.path);
    }
}

impl FromStr for MarkedPath<Absolute> {
    type Err = PathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Self::new(path)
    }
}

impl From<CanonicalPath> for MarkedPath<Absolute> {
    fn from(value: CanonicalPath) -> Self {
        value.into_marked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::path::Path;

    #[rstest]
    fn absolute_new_accepts_absolute_path() {
        // Given an absolute path.
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };

        // When creating a marked path.
        let result = MarkedPath::<Absolute>::new(path);

        // Then the result is ok.
        assert!(result.is_ok());
    }

    #[rstest]
    fn absolute_new_rejects_relative_path() {
        // Given a relative path.
        let path = PathBuf::from("some/relative/path");

        // When creating an absolute marked path.
        let result = MarkedPath::<Absolute>::new(path);

        // Then the result is an error.
        assert!(result.is_err());
    }

    #[rstest]
    fn push_path_on_absolute_accepts_relative() {
        // Given an absolute marked path and a relative marked path.
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let mut absolute = MarkedPath::<Absolute>::new(base_path).unwrap();
        let relative = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        // When pushing the relative path onto the absolute path.
        absolute.push(&relative);

        // Then the path is the combined result.
        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(absolute.as_path(), Path::new(expected));
    }
}
