use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::absolute::Absolute;
use crate::marked_path::{MarkedPath, PathError};

/// A wrapper for canonicalized absolute paths.
///
/// This type represents a path that has been resolved to its canonical form:
/// it is guaranteed to be absolute, with all `.` and `..` components resolved,
/// and all symbolic links followed. The path must exist on the filesystem
/// at the time of construction.
///
/// # Type Safety
///
/// A `CanonicalPath` provides stronger guarantees than [`MarkedPath<Absolute>`]:
/// - The path is absolute and fully resolved
/// - The path existed at construction time
/// - The path can be safely used for comparisons (no `.` or `..` ambiguity)
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use marked_path::CanonicalPath;
///
/// // Create from an existing path
/// let canonical = CanonicalPath::from_path(Path::new("/etc/hosts"))?;
/// println!("Canonical path: {}", canonical.as_path().display());
/// # Ok::<(), marked_path::PathError>(())
/// ```
#[derive(Debug, PartialOrd, Ord)]
pub struct CanonicalPath(MarkedPath<Absolute>);

impl Clone for CanonicalPath {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CanonicalPath {
    /// Creates a new `CanonicalPath` from a path, validating it is canonical.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if:
    /// - The path does not exist
    /// - The path is not in canonical form (contains `.`, `..`, or is a symlink)
    pub fn new(path: PathBuf) -> Result<Self, PathError> {
        let canonicalized = path.canonicalize()?;
        if canonicalized != path {
            return Err(PathError::NotCanonical);
        }
        Ok(Self(MarkedPath {
            path,
            _marker: PhantomData,
        }))
    }

    /// Creates a `CanonicalPath` by canonicalizing the given path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if the path cannot be canonicalized
    /// (e.g., if it doesn't exist or if there are permission issues).
    pub fn from_path(path: &Path) -> Result<Self, PathError> {
        let canonicalized = path.canonicalize()?;
        CanonicalPath::new(canonicalized)
    }

    /// Returns a reference to the underlying [`Path`].
    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    /// Returns a clone of the underlying [`PathBuf`].
    pub fn to_path_buf(&self) -> PathBuf {
        self.0.to_path_buf()
    }

    /// Consumes this `CanonicalPath` and returns the underlying [`PathBuf`].
    pub fn into_inner(self) -> PathBuf {
        self.0.into_inner()
    }

    /// Consumes this `CanonicalPath` and returns the inner [`MarkedPath<Absolute>`].
    pub(crate) fn into_marked(self) -> MarkedPath<Absolute> {
        self.0
    }
}

impl PartialEq for CanonicalPath {
    fn eq(&self, other: &Self) -> bool {
        self.0.path == other.0.path
    }
}

impl Eq for CanonicalPath {}

impl Hash for CanonicalPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.path.hash(state);
    }
}

impl AsRef<Path> for CanonicalPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl From<CanonicalPath> for PathBuf {
    fn from(value: CanonicalPath) -> Self {
        value.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile::NamedTempFile;

    #[rstest]
    fn canonical_path_from_existing_file() {
        // Given an existing file.
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // When creating a canonical path from the file path.
        let canonical = CanonicalPath::from_path(path);

        // Then the result is ok and the path is absolute.
        assert!(canonical.is_ok());
        let canonical = canonical.unwrap();
        assert!(canonical.as_path().is_absolute());
    }

    #[rstest]
    fn canonical_path_hash_and_eq() {
        // Given two canonical paths to the same file.
        use std::collections::HashSet;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let canonical1 = CanonicalPath::from_path(path).unwrap();
        let canonical2 = CanonicalPath::from_path(path).unwrap();

        // When comparing them and using in a HashSet.
        assert_eq!(canonical1, canonical2);

        let mut set = HashSet::new();
        set.insert(canonical1.clone());

        // Then they are equal and both hash to the same value.
        assert!(set.contains(&canonical2));
    }

    #[rstest]
    fn canonical_path_to_path_buf() {
        let temp_file = NamedTempFile::new().unwrap();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        assert_eq!(canonical.to_path_buf(), temp_file.path().to_path_buf());
    }

    #[rstest]
    fn canonical_path_into_inner() {
        let temp_file = NamedTempFile::new().unwrap();
        let expected = temp_file.path().to_path_buf();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        assert_eq!(canonical.into_inner(), expected);
    }

    #[rstest]
    fn canonical_path_not_equal_different_paths() {
        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();
        let canonical1 = CanonicalPath::from_path(temp_file1.path()).unwrap();
        let canonical2 = CanonicalPath::from_path(temp_file2.path()).unwrap();
        assert_ne!(canonical1, canonical2);
    }

    #[rstest]
    fn canonical_path_hash_different_paths() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();
        let canonical1 = CanonicalPath::from_path(temp_file1.path()).unwrap();
        let canonical2 = CanonicalPath::from_path(temp_file2.path()).unwrap();

        let mut hasher1 = DefaultHasher::new();
        canonical1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        canonical2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_ne!(
            hash1, hash2,
            "different canonical paths should produce different hashes"
        );
    }

    #[rstest]
    fn canonical_path_into_pathbuf() {
        let temp_file = NamedTempFile::new().unwrap();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        let pathbuf: PathBuf = canonical.into();
        assert_eq!(pathbuf, temp_file.path().to_path_buf());
    }
}
