use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::canonical::CanonicalPath;
use crate::marked_path::{MarkedPath, MarkedPathBuf, PathError};
use crate::relative::Relative;

/// Marker type for absolute paths.
///
/// This is a phantom marker type used with [`MarkedPath`] to indicate that
/// the contained path is guaranteed to be absolute. An absolute path starts
/// from the root of the filesystem (e.g., `/path/to/file` on Unix or
/// `C:\path\to\file` on Windows).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Absolute;

impl MarkedPathBuf<Absolute> {
    /// Creates a new `MarkedPathBuf<Absolute>` from the given path.
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

    /// Appends a relative path to this absolute path.
    pub fn push(&mut self, other: &MarkedPath<Relative>) {
        self.path.push(other.path);
    }
}

impl FromStr for MarkedPathBuf<Absolute> {
    type Err = PathError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'a> MarkedPath<'a, Absolute> {
    /// Creates a new borrowed `MarkedPath<Absolute>` from the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path is not absolute.
    pub fn new(path: &'a Path) -> Result<Self, PathError> {
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
    /// Returns a [`PathError`] if the path cannot be canonicalized.
    pub fn canonicalize(&self) -> Result<CanonicalPath, PathError> {
        let canonicalized = self.path.canonicalize()?;
        CanonicalPath::new(canonicalized)
    }

    /// Joins this absolute path with the given path, returning a new
    /// `MarkedPathBuf<Absolute>`.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NotAbsolute`] if the joined result is not absolute.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Result<MarkedPathBuf<Absolute>, PathError> {
        let joined = self.path.join(path);
        if joined.is_absolute() {
            Ok(MarkedPathBuf {
                path: joined,
                _marker: PhantomData,
            })
        } else {
            Err(PathError::NotAbsolute)
        }
    }

    /// Joins this absolute path with a typed relative path, returning a new
    /// `MarkedPathBuf<Absolute>`.
    ///
    /// This is infallible because joining a relative path onto an absolute
    /// path always produces an absolute path.
    pub fn join_relative(&self, other: &MarkedPath<Relative>) -> MarkedPathBuf<Absolute> {
        MarkedPathBuf {
            path: self.path.join(other.path),
            _marker: PhantomData,
        }
    }
}

impl From<CanonicalPath> for MarkedPathBuf<Absolute> {
    fn from(value: CanonicalPath) -> Self {
        value.into_marked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn absolute_new_accepts_absolute_path() {
        let path = if cfg!(windows) {
            PathBuf::from("C:\\some\\path")
        } else {
            PathBuf::from("/some/path")
        };

        let result = MarkedPathBuf::<Absolute>::new(path);

        assert!(result.is_ok());
    }

    #[rstest]
    fn absolute_new_rejects_relative_path() {
        let path = PathBuf::from("some/relative/path");

        let result = MarkedPathBuf::<Absolute>::new(path);

        assert!(result.is_err());
    }

    #[rstest]
    fn push_path_on_absolute_accepts_relative() {
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let mut absolute = MarkedPathBuf::<Absolute>::new(base_path).unwrap();
        let relative = MarkedPathBuf::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        absolute.push(&relative.as_marked_path());

        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(absolute.as_path(), Path::new(expected));
    }

    #[rstest]
    fn join_absolute_with_relative_raw() {
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let absolute = MarkedPathBuf::<Absolute>::new(base_path).unwrap();

        let result = absolute.as_marked_path().join("subdir/file.txt");

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
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let absolute = MarkedPathBuf::<Absolute>::new(base_path).unwrap();

        let other = if cfg!(windows) { "D:\\other" } else { "/other" };
        let result = absolute.as_marked_path().join(other);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_path(), Path::new(other));
    }

    #[rstest]
    fn join_relative_on_absolute() {
        let base_path = if cfg!(windows) {
            PathBuf::from("C:\\base")
        } else {
            PathBuf::from("/base")
        };
        let absolute = MarkedPathBuf::<Absolute>::new(base_path).unwrap();
        let relative = MarkedPathBuf::<Relative>::new(PathBuf::from("subdir/file.txt")).unwrap();

        let result = absolute
            .as_marked_path()
            .join_relative(&relative.as_marked_path());

        let expected = if cfg!(windows) {
            "C:\\base\\subdir\\file.txt"
        } else {
            "/base/subdir/file.txt"
        };
        assert_eq!(result.as_path(), Path::new(expected));
    }

    #[rstest]
    fn marked_path_new_accepts_absolute() {
        let path = if cfg!(windows) {
            Path::new(r"C:\some\path")
        } else {
            Path::new("/some/path")
        };
        let result = MarkedPath::<Absolute>::new(path);
        assert!(result.is_ok());
    }

    #[rstest]
    fn marked_path_new_rejects_relative() {
        let path = Path::new("some/relative/path");
        let result = MarkedPath::<Absolute>::new(path);
        assert!(result.is_err());
    }
}
