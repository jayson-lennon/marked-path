use std::marker::PhantomData;
use std::path::PathBuf;
use std::str::FromStr;

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
    pub fn new(path: PathBuf) -> Result<Self, PathError> {
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
}

impl FromStr for MarkedPath<Relative> {
    type Err = PathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Self::new(path)
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
}
