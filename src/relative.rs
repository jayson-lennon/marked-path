use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::canonical::CanonicalPath;
use crate::marked_path::{MarkedPath, MarkedPathBuf, PathError};

/// Marker type for relative paths.
///
/// This is a phantom marker type used with [`MarkedPath`] and [`MarkedPathBuf`]
/// to indicate that the contained path is guaranteed to be relative. A relative
/// path does not start from the root of the filesystem (e.g., `path/to/file`).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relative;

impl MarkedPathBuf<Relative> {
    /// Creates a new `MarkedPathBuf<Relative>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not relative.
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
        self.path.push(other.path);
    }
}

impl FromStr for MarkedPathBuf<Relative> {
    type Err = PathError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'a> MarkedPath<'a, Relative> {
    /// Creates a new borrowed `MarkedPath<Relative>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not relative.
    pub fn new(path: &'a Path) -> Result<Self, PathError> {
        if path.is_relative() {
            Ok(Self {
                path,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotRelative)
        }
    }

    /// Joins this relative path with the given path, returning a new
    /// `MarkedPathBuf<Relative>`.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotRelative`] if the joined result is not relative.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Result<MarkedPathBuf<Relative>, PathError> {
        let joined = self.path.join(path);
        if joined.is_relative() {
            Ok(MarkedPathBuf {
                path: joined,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotRelative)
        }
    }

    /// Joins this relative path with another typed relative path, returning a
    /// new `MarkedPathBuf<Relative>`.
    ///
    /// This is infallible because joining a relative path onto a relative
    /// path always produces a relative path.
    pub fn join_relative(&self, other: &MarkedPath<Relative>) -> MarkedPathBuf<Relative> {
        MarkedPathBuf {
            path: self.path.join(other.path),
            _marker: PhantomData,
        }
    }

    /// Canonicalizes this relative path, returning a [`CanonicalPath`].
    ///
    /// The relative path is resolved against the current working directory.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path cannot be canonicalized.
    pub fn canonicalize(&self) -> Result<CanonicalPath, PathError> {
        let canonicalized = self.path.canonicalize()?;
        CanonicalPath::new(canonicalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn relative_new_accepts_relative_path() {
        let path = PathBuf::from("some/relative/path");
        let result = MarkedPathBuf::<Relative>::new(path);
        assert!(result.is_ok());
    }

    #[rstest]
    fn relative_new_accepts_empty_path() {
        let result = MarkedPathBuf::<Relative>::new("");
        assert!(result.is_ok());
    }

    #[rstest]
    fn relative_new_rejects_absolute_path() {
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };
        let result = MarkedPathBuf::<Relative>::new(path);
        assert!(result.is_err());
    }

    #[rstest]
    fn marked_path_new_accepts_relative() {
        let path = Path::new("some/relative/path");
        let result = MarkedPath::<Relative>::new(path);
        assert!(result.is_ok());
    }

    #[rstest]
    fn marked_path_new_rejects_absolute() {
        let path = if cfg!(windows) {
            Path::new(r"C:\some\path")
        } else {
            Path::new("/some/path")
        };
        let result = MarkedPath::<Relative>::new(path);
        assert!(result.is_err());
    }

    #[rstest]
    fn push_path_on_relative_accepts_relative() {
        let mut base = MarkedPathBuf::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = MarkedPathBuf::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();
        base.push(&other.as_marked_path());
        assert_eq!(base.as_path(), std::path::Path::new("base/subdir/file.txt"));
    }

    #[rstest]
    fn join_relative_with_relative_raw() {
        let base = MarkedPathBuf::<Relative>::new(PathBuf::from("base")).unwrap();
        let result = base.as_marked_path().join("subdir/file.txt");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().as_path(),
            std::path::Path::new("base/subdir/file.txt")
        );
    }

    #[rstest]
    fn join_relative_rejects_absolute_raw() {
        let base = MarkedPathBuf::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = if cfg!(windows) { "C:\\other" } else { "/other" };
        let result = base.as_marked_path().join(other);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PathError::NotRelative));
    }

    #[rstest]
    fn join_relative_on_relative() {
        let base = MarkedPathBuf::<Relative>::new(PathBuf::from("base")).unwrap();
        let other = MarkedPathBuf::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();
        let result = base.as_marked_path().join_relative(&other.as_marked_path());
        assert_eq!(
            result.as_path(),
            std::path::Path::new("base/subdir/file.txt")
        );
    }

    #[rstest]
    fn canonicalize_relative_path() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("marked_path_test_canonicalize_dir");
        std::fs::create_dir_all(&test_dir).unwrap();
        let file_path = test_dir.join("test_file.txt");
        std::fs::write(&file_path, "test").unwrap();

        let prev_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&test_dir).unwrap();

        let relative = std::path::PathBuf::from("test_file.txt");
        let result = MarkedPathBuf::<Relative>::new(relative)
            .unwrap()
            .as_marked_path()
            .canonicalize();

        assert!(result.is_ok());
        let canonical = result.unwrap();
        assert_eq!(canonical.as_path(), file_path.canonicalize().unwrap());

        std::env::set_current_dir(&prev_dir).unwrap();
        std::fs::remove_file(&file_path).ok();
        std::fs::remove_dir(&test_dir).ok();
    }
}
