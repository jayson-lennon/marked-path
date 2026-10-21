// Copyright (C) 2026 Jayson Lennon
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program; if not, see <https://opensource.org/license/lgpl-3-0>.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Ancestors, Path, PathBuf};

use serde::de::{self, Deserializer, IgnoredAny, MapAccess};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use wherror::Error;

use crate::marker::Marker;
use crate::normalize::normalize_lexical_path;
use crate::path_access::{MarkedPathAccess, PathWrapper};

/// Error type for path operations.
///
/// This error is returned when a path operation fails, such as attempting to
/// create a `MarkedPathBuf<Absolute>` from a relative path, or vice versa.
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

    /// The path does not start with the base it was to be stripped against.
    #[error("path does not start with the given prefix")]
    PrefixMismatch,

    /// An I/O error occurred during path operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// An owned, type-safe path wrapper with a zero-sized marker.
///
/// See [`MarkedPath`] for the borrowed equivalent.
///
/// # Serialization
///
/// Serializes as a two-field object — the path and the marker that
/// constructed it:
///
/// ```json
/// {"path": "/home/user", "marker": "absolute"}
/// ```
///
/// The marker is a *label*, checked against the target type on read, never
/// trusted: a value only deserializes into [`MarkedPathBuf<Absolute>`] if its
/// content actually is absolute, whatever the label claims. The bare-string
/// shape `"/home/user"` — the shape every pre-marker artifact already stores
/// — is accepted the same way, so old data loads unchanged. Fields other than
/// `path` and `marker` are ignored, keeping the format open to new fields.
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkedPathBuf<M> {
    pub(crate) path: PathBuf,
    pub(crate) _marker: PhantomData<M>,
}

impl<M: Marker> Serialize for MarkedPathBuf<M> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("MarkedPathBuf", 2)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("marker", M::NAME)?;
        state.end()
    }
}

impl<'de, M: Marker> Deserialize<'de> for MarkedPathBuf<M> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(MarkedPathBufVisitor::default())
    }
}

/// Deserialization visitor accepting both wire shapes.
///
/// The label is checked the moment it is read and the content is validated
/// against [`Marker::validate`] at the end, so a mistyped path fails at the
/// boundary instead of quietly constructing a wrapper that violates the
/// marker's invariant.
struct MarkedPathBufVisitor<M> {
    _marker: PhantomData<M>,
}

impl<M> Default for MarkedPathBufVisitor<M> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<M: Marker> MarkedPathBufVisitor<M> {
    fn wrap(path: PathBuf) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path,
            _marker: PhantomData,
        }
    }
}

impl<'de, M: Marker> de::Visitor<'de> for MarkedPathBufVisitor<M> {
    type Value = MarkedPathBuf<M>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a path string or a {{path, marker}} object marked `{}`",
            M::NAME
        )
    }

    /// Legacy bare-string shape.
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        let path = Path::new(value);
        M::validate(path).map_err(E::custom)?;
        Ok(Self::wrap(path.to_path_buf()))
    }

    /// Tagged object shape: `{path, marker}`.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut path: Option<PathBuf> = None;
        let mut marker_seen = false;

        while let Some(field) = map.next_key::<WireField>()? {
            match field {
                WireField::Path => {
                    if path.is_some() {
                        return Err(de::Error::duplicate_field("path"));
                    }
                    path = Some(map.next_value()?);
                }
                WireField::Marker => {
                    if marker_seen {
                        return Err(de::Error::duplicate_field("marker"));
                    }
                    marker_seen = true;
                    let label = map.next_value::<String>()?;
                    if label != M::NAME {
                        return Err(de::Error::custom(format!(
                            "path marker `{label}` does not match expected `{}`",
                            M::NAME
                        )));
                    }
                }
                WireField::Other => {
                    // Unknown fields are read and dropped, so adding a field
                    // to the wire format does not break older readers.
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
        if !marker_seen {
            return Err(de::Error::missing_field("marker"));
        }
        M::validate(&path).map_err(de::Error::custom)?;
        Ok(Self::wrap(path))
    }
}

/// Field identifiers of the tagged object shape.
enum WireField {
    Path,
    Marker,
    Other,
}

impl<'de> Deserialize<'de> for WireField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = WireField;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("`path` or `marker`")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match value {
                    "path" => Ok(WireField::Path),
                    "marker" => Ok(WireField::Marker),
                    _ => Ok(WireField::Other),
                }
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)
    }
}

impl<M> Clone for MarkedPathBuf<M> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _marker: PhantomData,
        }
    }
}

impl<M: Marker> fmt::Debug for MarkedPathBuf<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", M::LABEL, self.path)
    }
}

impl<M> fmt::Display for MarkedPathBuf<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<M> AsRef<Path> for MarkedPathBuf<M> {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Dereferences to the underlying [`Path`], so the marked types can be used
/// wherever a path is expected without an explicit conversion.
impl<M> Deref for MarkedPathBuf<M> {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl<M> From<MarkedPathBuf<M>> for PathBuf {
    fn from(value: MarkedPathBuf<M>) -> Self {
        value.path
    }
}

impl<M> PathWrapper for MarkedPathBuf<M> {
    type Owned = MarkedPathBuf<M>;
    type Borrowed<'a>
        = MarkedPath<'a, M>
    where
        M: 'a;
    type Ancestors<'a>
        = MarkedAncestors<'a, M>
    where
        M: 'a;

    fn wrap_buf(path: PathBuf) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path,
            _marker: PhantomData,
        }
    }

    fn wrap_ref(path: &Path) -> MarkedPath<'_, M> {
        MarkedPath {
            path,
            _marker: PhantomData,
        }
    }
}

impl<M> MarkedPathAccess for MarkedPathBuf<M> {
    fn as_path(&self) -> &Path {
        &self.path
    }

    fn ancestors(&self) -> MarkedAncestors<'_, M> {
        MarkedAncestors {
            inner: self.path.ancestors(),
            _marker: PhantomData,
        }
    }
}

impl<M> MarkedPathBuf<M> {
    /// Consumes this `MarkedPathBuf` and returns the underlying [`PathBuf`].
    pub fn into_inner(self) -> PathBuf {
        self.path
    }

    /// Consumes this `MarkedPathBuf` and returns the underlying [`OsString`].
    ///
    /// See [`PathBuf::into_os_string`](std::path::PathBuf::into_os_string).
    pub fn into_os_string(self) -> OsString {
        self.path.into_os_string()
    }

    /// Returns a borrowed [`MarkedPath`] view of this path.
    pub fn as_marked_path(&self) -> MarkedPath<'_, M> {
        MarkedPath {
            path: &self.path,
            _marker: PhantomData,
        }
    }

    /// Truncates this path to its parent.
    ///
    /// See [`PathBuf::pop`](std::path::PathBuf::pop).
    pub fn pop(&mut self) -> bool {
        self.path.pop()
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

    /// Returns a lexically normalized copy of this path.
    ///
    /// `.` segments are dropped; `..` pops the preceding component, is
    /// dropped at a root (a root has no parent), and is kept when a relative
    /// path has nothing left to pop (`../a` stays `../a`). The marker is
    /// preserved: normalizing a `MarkedPathBuf<Absolute>` cannot make it
    /// relative, and vice versa.
    ///
    /// This is purely lexical — no filesystem access, so symlinks are neither
    /// resolved nor detected. Normalizing the lone relative path `.` yields
    /// the empty path, which joins treat as the current directory.
    #[must_use = "returning a new MarkedPathBuf without using it is likely a mistake"]
    pub fn normalize_lexical(&self) -> MarkedPathBuf<M> {
        <Self as PathWrapper>::wrap_buf(normalize_lexical_path(&self.path))
    }
}

/// A borrowed, type-safe path reference with a zero-sized marker.
///
/// See [`MarkedPathBuf`] for the owned equivalent.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarkedPath<'a, M> {
    pub(crate) path: &'a Path,
    pub(crate) _marker: PhantomData<M>,
}

impl<M: Marker> fmt::Debug for MarkedPath<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", M::LABEL, self.path)
    }
}

impl<M> fmt::Display for MarkedPath<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.display().fmt(f)
    }
}

impl<M> AsRef<Path> for MarkedPath<'_, M> {
    fn as_ref(&self) -> &Path {
        self.path
    }
}

/// Dereferences to the underlying [`Path`], so the marked types can be used
/// wherever a path is expected without an explicit conversion.
impl<M> Deref for MarkedPath<'_, M> {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.path
    }
}

impl<M> PathWrapper for MarkedPath<'_, M> {
    type Owned = MarkedPathBuf<M>;
    type Borrowed<'b>
        = MarkedPath<'b, M>
    where
        Self: 'b;
    type Ancestors<'b>
        = MarkedAncestors<'b, M>
    where
        Self: 'b;

    fn wrap_buf(path: PathBuf) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path,
            _marker: PhantomData,
        }
    }

    fn wrap_ref(path: &Path) -> MarkedPath<'_, M> {
        MarkedPath {
            path,
            _marker: PhantomData,
        }
    }
}

impl<'a, M> MarkedPathAccess for MarkedPath<'a, M> {
    fn as_path(&self) -> &'a Path {
        self.path
    }

    fn ancestors(&self) -> MarkedAncestors<'a, M> {
        MarkedAncestors {
            inner: self.path.ancestors(),
            _marker: PhantomData,
        }
    }
}

impl<M> MarkedPath<'_, M> {
    /// Returns an owned [`MarkedPathBuf`] cloning the path data.
    pub fn to_owned(&self) -> MarkedPathBuf<M> {
        MarkedPathBuf {
            path: self.path.to_path_buf(),
            _marker: PhantomData,
        }
    }
}

/// An iterator over [`MarkedPath`] ancestors.
///
/// See [`Path::ancestors`](std::path::Path::ancestors).
pub struct MarkedAncestors<'a, M> {
    inner: Ancestors<'a>,
    _marker: PhantomData<M>,
}

impl<'a, M> Iterator for MarkedAncestors<'a, M> {
    type Item = MarkedPath<'a, M>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|p| MarkedPath {
            path: p,
            _marker: PhantomData,
        })
    }
}

impl<'a, M> From<std::path::Ancestors<'a>> for MarkedAncestors<'a, M> {
    fn from(inner: std::path::Ancestors<'a>) -> Self {
        MarkedAncestors {
            inner,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::absolute::Absolute;
    use crate::relative::Relative;
    use rstest::rstest;

    #[rstest]
    fn marked_path_display() {
        let path =
            MarkedPathBuf::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(format!("{path}"), "some/relative/path");
    }

    #[rstest]
    fn marked_path_to_path_buf() {
        let path =
            MarkedPathBuf::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(
            path.to_path_buf(),
            std::path::PathBuf::from("some/relative/path")
        );
    }

    #[rstest]
    fn marked_path_into_inner() {
        let path =
            MarkedPathBuf::<Relative>::new(std::path::PathBuf::from("some/relative/path")).unwrap();
        assert_eq!(
            path.into_inner(),
            std::path::PathBuf::from("some/relative/path")
        );
    }

    #[rstest]
    fn marked_path_parent_on_relative() {
        let buf = MarkedPathBuf::<Relative>::new("a/b/c").unwrap();
        let parent = buf.parent().unwrap();
        assert_eq!(parent.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn marked_path_parent_on_absolute() {
        let buf = MarkedPathBuf::<Absolute>::new("/a/b/c").unwrap();
        let parent = buf.parent().unwrap();
        assert_eq!(parent.as_path(), std::path::Path::new("/a/b"));
    }

    #[rstest]
    fn marked_path_parent_on_root() {
        let buf = MarkedPathBuf::<Absolute>::new("/").unwrap();
        assert!(buf.parent().is_none());
    }

    #[rstest]
    fn marked_path_parent_on_single_component() {
        let buf = MarkedPathBuf::<Relative>::new("foo").unwrap();
        let parent = buf.parent().unwrap();
        assert_eq!(parent.as_path(), std::path::Path::new(""));
    }

    #[rstest]
    fn marked_path_parent_on_empty() {
        let buf = MarkedPathBuf::<Relative>::new("").unwrap();
        assert!(buf.parent().is_none());
    }

    #[rstest]
    fn marked_ancestors_yields_typed_paths() {
        let buf = MarkedPathBuf::<Relative>::new("a/b/c").unwrap();
        let ancestors: Vec<_> = buf.ancestors().collect();
        assert_eq!(ancestors.len(), 4);
        assert_eq!(ancestors[0].as_path(), std::path::Path::new("a/b/c"));
        assert_eq!(ancestors[1].as_path(), std::path::Path::new("a/b"));
        assert_eq!(ancestors[2].as_path(), std::path::Path::new("a"));
        assert_eq!(ancestors[3].as_path(), std::path::Path::new(""));
    }

    #[rstest]
    fn deref_from_buf_to_path() {
        let buf: MarkedPathBuf<Relative> = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let borrowed: MarkedPath<Relative> = MarkedPath {
            path: buf.as_path(),
            _marker: PhantomData,
        };
        assert_eq!(borrowed.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn to_owned_from_borrowed() {
        let buf = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let borrowed: MarkedPath<Relative> = MarkedPath {
            path: buf.as_path(),
            _marker: PhantomData,
        };
        let owned = borrowed.to_owned();
        assert_eq!(owned.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn marked_path_into_pathbuf() {
        let path = MarkedPathBuf::<Relative>::new("some/relative/path").unwrap();
        let pathbuf: PathBuf = path.into();
        assert_eq!(pathbuf, PathBuf::from("some/relative/path"));
    }

    #[rstest]
    fn marked_path_buf_derefs_to_path() {
        let buf = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        assert_eq!(&*buf, std::path::Path::new("a/b"));
    }

    #[rstest]
    fn marked_path_derefs_to_path() {
        let buf = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let borrowed = buf.as_marked_path();
        assert_eq!(&*borrowed, std::path::Path::new("a/b"));
    }

    #[rstest]
    fn debug_on_owned_prints_marker_and_quoted_path() {
        // Given an absolute marked path and a relative one.
        let abs = test_absolute();
        let rel = MarkedPathBuf::<Relative>::new("a/b").unwrap();

        // When formatting them with Debug.
        let abs_text = format!("{abs:?}");
        let rel_text = format!("{rel:?}");

        // Then the marker name leads and the path follows in Path's own
        // quoted form.
        if cfg!(windows) {
            assert_eq!(abs_text, r#"Absolute("C:\\tmp\\x")"#);
        } else {
            assert_eq!(abs_text, r#"Absolute("/tmp/x")"#);
        }
        assert_eq!(rel_text, r#"Relative("a/b")"#);
    }

    #[rstest]
    fn debug_on_borrowed_prints_marker_and_quoted_path() {
        // Given a borrowed view of a marked path.
        let buf = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let borrowed = buf.as_marked_path();

        // When formatting it with Debug.
        let text = format!("{borrowed:?}");

        // Then it reads the same as the owned form.
        assert_eq!(text, r#"Relative("a/b")"#);
    }

    #[rstest]
    fn debug_escapes_like_the_underlying_path_debug() {
        // Given a path containing characters Path's Debug escapes.
        let rel = MarkedPathBuf::<Relative>::new("a b/c\td").unwrap();

        // When formatting it with Debug.
        let text = format!("{rel:?}");

        // Then the quoted portion matches `Path`'s own Debug byte for byte.
        let inner = format!("{:?}", std::path::Path::new("a b/c\td"));
        assert_eq!(text, format!("Relative({inner})"));
    }

    #[rstest]
    fn display_stays_the_bare_path() {
        // Given a marked path.
        let rel = MarkedPathBuf::<Relative>::new("a/b").unwrap();

        // When formatting it with Display.
        let text = format!("{rel}");

        // Then no marker leaks into user-facing output.
        assert_eq!(text, "a/b");
    }

    #[rstest]
    #[case("a/./b", "a/b")]
    #[case("a/b/../c", "a/c")]
    #[case("../a", "../a")]
    #[case("a/../../b", "../b")]
    #[case(".", "")]
    #[case("a/b", "a/b")]
    fn relative_normalize_lexical_table(#[case] input: &str, #[case] expected: &str) {
        // Given a relative marked path.
        let rel = MarkedPathBuf::<Relative>::new(input).unwrap();

        // When normalizing it lexically.
        let normalized = rel.normalize_lexical();

        // Then dot segments resolve per the reference table and the marker
        // stays Relative.
        assert_eq!(normalized.as_path(), Path::new(expected));
    }

    #[rstest]
    #[case("/x/../y", "/y")]
    #[case("/../..", "/")]
    fn absolute_normalize_lexical_table(#[case] input: &str, #[case] expected: &str) {
        // Given an absolute marked path with parent segments.
        let abs = MarkedPathBuf::<Absolute>::new(input).unwrap();

        // When normalizing it lexically.
        let normalized = abs.normalize_lexical();

        // Then parents clamp at the root and the marker stays Absolute.
        assert_eq!(normalized.as_path(), Path::new(expected));
    }

    #[rstest]
    fn normalize_lexical_preserves_the_marker_type() {
        // Given a relative path that normalization would leave non-empty.
        let rel = MarkedPathBuf::<Relative>::new("a/../b").unwrap();

        // When normalizing it.
        let normalized: MarkedPathBuf<Relative> = rel.normalize_lexical();

        // Then the result still carries the Relative marker — normalization
        // never re-types the path.
        assert_eq!(normalized.as_path(), Path::new("b"));
    }

    fn test_absolute() -> MarkedPathBuf<Absolute> {
        if cfg!(windows) {
            MarkedPathBuf::<Absolute>::new(r"C:\tmp\x").unwrap()
        } else {
            MarkedPathBuf::<Absolute>::new("/tmp/x").unwrap()
        }
    }

    fn test_absolute_wire() -> String {
        if cfg!(windows) {
            r#"{"path":"C:\\tmp\\x","marker":"absolute"}"#.to_owned()
        } else {
            r#"{"path":"/tmp/x","marker":"absolute"}"#.to_owned()
        }
    }

    #[rstest]
    fn absolute_serializes_as_path_and_marker_object() {
        // Given an absolute marked path.
        let abs = test_absolute();

        // When serializing it.
        let wire = serde_json::to_string(&abs).unwrap();

        // Then it is a two-field object naming the marker, and no field is
        // ever null.
        assert_eq!(wire, test_absolute_wire());
    }

    #[rstest]
    fn relative_serializes_with_relative_marker() {
        // Given a relative marked path.
        let rel = MarkedPathBuf::<Relative>::new("a/b").unwrap();

        // When serializing it.
        let wire = serde_json::to_string(&rel).unwrap();

        // Then the marker field carries the relative label.
        assert_eq!(wire, r#"{"path":"a/b","marker":"relative"}"#);
    }

    #[rstest]
    fn empty_relative_path_roundtrips() {
        // Given an empty relative path (std-consistent: empty is relative).
        let rel = MarkedPathBuf::<Relative>::new("").unwrap();

        // When it is serialized and deserialized again.
        let wire = serde_json::to_string(&rel).unwrap();
        let loaded: MarkedPathBuf<Relative> = serde_json::from_str(&wire).unwrap();

        // Then the content survives byte-identical.
        assert_eq!(wire, r#"{"path":"","marker":"relative"}"#);
        assert_eq!(loaded.as_path(), std::path::Path::new(""));
    }

    #[rstest]
    fn bare_string_deserializes_as_absolute() {
        // Given a bare path string in the pre-marker wire shape.
        let wire = if cfg!(windows) {
            r#""C:\\tmp\\x""#
        } else {
            r#""/tmp/x""#
        };

        // When deserializing it as an absolute marked path.
        let loaded: Result<MarkedPathBuf<Absolute>, _> = serde_json::from_str(wire);

        // Then the legacy shape is accepted.
        let loaded = loaded.unwrap();
        assert_eq!(loaded.as_path(), test_absolute().as_path());
    }

    #[rstest]
    fn bare_relative_string_rejected_for_absolute() {
        // Given a bare string holding a relative path.
        let wire = r#""a/b""#;

        // When deserializing it as an absolute marked path.
        let result: Result<MarkedPathBuf<Absolute>, _> = serde_json::from_str(wire);

        // Then the content check rejects it.
        assert!(result.is_err());
    }

    #[rstest]
    fn tagged_relative_content_rejected_for_absolute() {
        // Given a tagged object whose content is relative but whose label
        // claims absolute.
        let wire = r#"{"path":"a/b","marker":"absolute"}"#;

        // When deserializing it as an absolute marked path.
        let result: Result<MarkedPathBuf<Absolute>, _> = serde_json::from_str(wire);

        // Then the content check rejects it despite the label.
        assert!(result.is_err());
    }

    #[rstest]
    fn mislabeled_marker_rejected_for_absolute() {
        // Given a tagged object with absolute content but a relative label.
        let wire = if cfg!(windows) {
            r#"{"path":"C:\\tmp\\x","marker":"relative"}"#
        } else {
            r#"{"path":"/tmp/x","marker":"relative"}"#
        };

        // When deserializing it as an absolute marked path.
        let result: Result<MarkedPathBuf<Absolute>, _> = serde_json::from_str(wire);

        // Then the label check rejects it before the content is even read.
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not match expected `absolute`"),
            "unexpected error: {err}"
        );
    }

    #[rstest]
    fn bare_string_deserializes_as_relative() {
        // Given a bare string holding a relative path.
        let wire = r#""a/b""#;

        // When deserializing it as a relative marked path.
        let loaded: MarkedPathBuf<Relative> = serde_json::from_str(wire).unwrap();

        // Then the legacy shape is accepted.
        assert_eq!(loaded.as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn absolute_string_rejected_for_relative() {
        // Given a bare string holding an absolute path.
        let wire = if cfg!(windows) {
            r#""C:\\tmp\\x""#
        } else {
            r#""/tmp/x""#
        };

        // When deserializing it as a relative marked path.
        let result: Result<MarkedPathBuf<Relative>, _> = serde_json::from_str(wire);

        // Then the content check rejects it.
        assert!(result.is_err());
    }

    #[rstest]
    fn drive_prefix_string_rejected_for_relative() {
        // Given a bare string holding a drive-relative path, which resolves
        // against that drive's working directory rather than the process's.
        let wire = r#""C:foo""#;

        // When deserializing it as a relative marked path.
        let result: Result<MarkedPathBuf<Relative>, _> = serde_json::from_str(wire);

        // Then the drive-prefix rule rejects it on every platform.
        assert!(result.is_err());
    }

    #[rstest]
    fn tagged_absolute_roundtrips_content() {
        // Given an absolute marked path serialized to its tagged form.
        let abs = test_absolute();
        let wire = serde_json::to_string(&abs).unwrap();

        // When deserializing it back as an absolute marked path.
        let loaded: MarkedPathBuf<Absolute> = serde_json::from_str(&wire).unwrap();

        // Then the content is byte-identical.
        assert_eq!(loaded.as_path(), abs.as_path());
    }

    #[rstest]
    fn tagged_relative_roundtrips_content() {
        // Given a relative marked path serialized to its tagged form.
        let rel = MarkedPathBuf::<Relative>::new("a/b").unwrap();
        let wire = serde_json::to_string(&rel).unwrap();

        // When deserializing it back as a relative marked path.
        let loaded: MarkedPathBuf<Relative> = serde_json::from_str(&wire).unwrap();

        // Then the content is byte-identical.
        assert_eq!(loaded.as_path(), rel.as_path());
    }

    #[rstest]
    fn unknown_field_is_ignored() {
        // Given a tagged object carrying a field this version does not know.
        let wire = r#"{"path":"a/b","marker":"relative","future":"anything"}"#;

        // When deserializing it as a relative marked path.
        let loaded: Result<MarkedPathBuf<Relative>, _> = serde_json::from_str(wire);

        // Then the unknown field does not block the read.
        assert_eq!(loaded.unwrap().as_path(), std::path::Path::new("a/b"));
    }

    #[rstest]
    fn missing_marker_field_rejected() {
        // Given a tagged object without the marker field.
        let wire = r#"{"path":"a/b"}"#;

        // When deserializing it as a relative marked path.
        let result: Result<MarkedPathBuf<Relative>, _> = serde_json::from_str(wire);

        // Then the object form requires the label.
        assert!(result.is_err());
    }

    #[rstest]
    fn missing_path_field_rejected() {
        // Given a tagged object without the path field.
        let wire = r#"{"marker":"relative"}"#;

        // When deserializing it as a relative marked path.
        let result: Result<MarkedPathBuf<Relative>, _> = serde_json::from_str(wire);

        // Then the object form requires the content.
        assert!(result.is_err());
    }

    #[rstest]
    fn duplicate_path_field_rejected() {
        // Given a tagged object repeating the path field.
        let wire = r#"{"path":"a","path":"b","marker":"relative"}"#;

        // When deserializing it as a relative marked path.
        let result: Result<MarkedPathBuf<Relative>, _> = serde_json::from_str(wire);

        // Then the duplicate is an error rather than a silent last-wins.
        assert!(result.is_err());
    }
}
