use std::marker::PhantomData;
use std::path::PathBuf;
use std::str::FromStr;

use crate::canonical::CanonicalPath;
use crate::marked_path::{MarkedPath, PathError};

/// Marker type for relative paths.
///
/// This is a phantom marker type used with [`MarkedPath`] to indicate that
/// the contained path is guaranteed to be relative. A relative path does not
/// start from the root of the filesystem (e.g., `path/to/file`).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relative;

impl MarkedPath<Relative> {
    /// Creates a new `MarkedPath<Relative>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not relative (i.e., if it's absolute).
    pub fn new<P>(path: P) -> Result<Self, PathError>
    where
        P: Into<PathBuf>,
    {
        let path = path.into();
        if path.is_relative() {
            Ok(Self {
                path,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotRelative)
        }
    }

    /// Appends another relative path to this relative path.
    pub fn push(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(&other.path);
    }

    /// Joins this relative path with the given path, returning a new
    /// `MarkedPath<Relative>`.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotRelative`] if the joined result is not relative
    /// (e.g., if an absolute path was passed).
    pub fn join<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<MarkedPath<Relative>, PathError> {
        let joined = self.path.join(path);
        if joined.is_relative() {
            Ok(MarkedPath {
                path: joined,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotRelative)
        }
    }

    /// Joins this relative path with another typed relative path, returning a
    /// new `MarkedPath<Relative>`.
    ///
    /// This is infallible because joining a relative path onto a relative
    /// path always produces a relative path.
    pub fn join_relative(&self, other: &MarkedPath<Relative>) -> MarkedPath<Relative> {
        MarkedPath {
            path: self.path.join(&other.path),
            _marker: PhantomData,
        }
    }

    /// Canonicalizes this relative path, returning a [`CanonicalPath`].
    ///
    /// The relative path is resolved against the current working directory.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path cannot be canonicalized
    /// (e.g., if it doesn't exist or there are permission issues).
    pub fn canonicalize(&self) -> Result<CanonicalPath, PathError> {
        let canonicalized = self.path.canonicalize()?;
        CanonicalPath::new(canonicalized)
    }
}

impl FromStr for MarkedPath<Relative> {
    type Err = PathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn relative_new_accepts_relative_path() {
        // Given a relative path.
        let path = PathBuf::from("some/relative/path");

        // When creating a relative marked path.
        let result = MarkedPath::<Relative>::new(path);

        // Then the result is ok.
        assert!(result.is_ok());
    }

    #[rstest]
    fn relative_new_accepts_empty_path() {
        // Given an empty path, which std considers relative.
        let result = MarkedPath::<Relative>::new("");

        // Then the result is ok.
        assert!(result.is_ok());
    }

    #[rstest]
    fn relative_new_rejects_absolute_path() {
        // Given an absolute path.
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };

        // When creating a relative marked path.
        let result = MarkedPath::<Relative>::new(path);

        // Then the result is an error.
        assert!(result.is_err());
    }

    #[rstest]
    fn push_path_on_relative_accepts_relative() {
        // Given two relative marked paths.
        let mut base = MarkedPath::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        // When pushing one path onto the other.
        base.push(&other);

        // Then the path is the combined result.
        assert_eq!(base.as_path(), std::path::Path::new("base/subdir/file.txt"));
    }

    #[rstest]
    fn join_relative_with_relative_raw() {
        // Given a relative marked path.
        let base = MarkedPath::<Relative>::new(PathBuf::from("base")).unwrap();

        // When joining with a relative path.
        let result = base.join("subdir/file.txt");

        // Then the result is ok and the path is combined.
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().as_path(),
            std::path::Path::new("base/subdir/file.txt")
        );
    }

    #[rstest]
    fn join_relative_rejects_absolute_raw() {
        // Given a relative marked path.
        let base = MarkedPath::<Relative>::new(PathBuf::from("base")).unwrap();

        // When joining with an absolute path.
        let other = if cfg!(windows) { "C:\\other" } else { "/other" };
        let result = base.join(other);

        // Then the result is an error.
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PathError::NotRelative));
    }

    #[rstest]
    fn join_relative_on_relative() {
        // Given two relative marked paths.
        let base = MarkedPath::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = MarkedPath::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        // When joining.
        let result = base.join_relative(&other);

        // Then the path is combined.
        assert_eq!(
            result.as_path(),
            std::path::Path::new("base/subdir/file.txt")
        );
    }

    #[rstest]
    fn canonicalize_relative_path() {
        // Given a temporary file in a subdirectory of temp.
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("marked_path_test_canonicalize_dir");
        std::fs::create_dir_all(&test_dir).unwrap();
        let file_path = test_dir.join("test_file.txt");
        std::fs::write(&file_path, "test").unwrap();

        // Save current directory and change to the test directory.
        let prev_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&test_dir).unwrap();

        // Given a relative path to the file.
        let relative = std::path::PathBuf::from("test_file.txt");

        // When canonicalizing.
        let result = MarkedPath::<Relative>::new(relative)
            .unwrap()
            .canonicalize();

        // Then the result is ok and matches the canonical path.
        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert_eq!(canonical.as_path(), file_path.canonicalize().unwrap());

        // Cleanup.
        std::env::set_current_dir(&prev_dir).unwrap();
        std::fs::remove_file(&file_path).ok();
        std::fs::remove_dir(&test_dir).ok();
    }
}
