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
    pub fn new<P>(path: P) -> Result<Self, PathError>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
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

    /// Joins this absolute path with the given path, returning a new
    /// `MarkedPath<Absolute>`.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotAbsolute`] if the joined result is not absolute
    /// (e.g., if an absolute path was passed that would replace the base).
    pub fn join<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<MarkedPath<Absolute>, PathError> {
        let joined = self.path.join(path);
        if joined.is_absolute() {
            Ok(MarkedPath {
                path: joined,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotAbsolute)
        }
    }

    /// Joins this absolute path with a typed relative path, returning a new
    /// `MarkedPath<Absolute>`.
    ///
    /// This is infallible because joining a relative path onto an absolute
    /// path always produces an absolute path.
    pub fn join_relative(&self, other: &MarkedPath<Relative>) -> MarkedPath<Absolute> {
        MarkedPath {
            path: self.path.join(&other.path),
            _marker: PhantomData,
        }
    }
}

impl FromStr for MarkedPath<Absolute> {
    type Err = PathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
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

    #[rstest]
    fn join_absolute_with_relative_raw() {
        // Given an absolute marked path.
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let absolute = MarkedPath::<Absolute>::new(base_path).unwrap();

        // When joining with a relative path.
        let result = absolute.join("subdir/file.txt");

        // Then the result is ok and the path is combined.
        assert!(result.is_ok());
        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(result.unwrap().as_path(), Path::new(expected));
    }

    #[rstest]
    fn join_absolute_with_absolute_raw_replaces_base() {
        // Given an absolute marked path.
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let absolute = MarkedPath::<Absolute>::new(base_path).unwrap();

        // When joining with an absolute path (which replaces the base).
        let other = if cfg!(windows) { "D:\\other" } else { "/other" };
        let result = absolute.join(other);

        // Then the result succeeds and is the replacement path.
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_path(), Path::new(other));
    }

    #[rstest]
    fn join_relative_on_absolute() {
        // Given an absolute marked path and a relative marked path.
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let absolute = MarkedPath::<Absolute>::new(base_path).unwrap();
        let relative = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        // When joining.
        let result = absolute.join_relative(&relative);

        // Then the path is combined.
        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(result.as_path(), Path::new(expected));
    }
}
