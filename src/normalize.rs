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

//! Cross-platform path normalization.
//!
//! Three transforms have to happen before a path can be stored in a marked
//! type, because the standard library's notion of "the same path" differs
//! between platforms:
//!
//! - **Separator style.** A relative path is normally written with forward
//!   slashes (`sub/dir/file.txt`) because that reads the same everywhere.
//!   Windows accepts both `/` and `\`, so the forward slashes survive a join
//!   and leave a mixed-separator path behind. On Windows the forward slashes
//!   are rewritten to the native backslash. On Unix `/` is the *only*
//!   separator — `\` is an ordinary character in a file name — so the path is
//!   left untouched.
//!
//! - **Drive prefixes.** A Windows drive prefix (`C:`, `C:foo`) makes a path
//!   *drive-relative*: it resolves against the working directory of that
//!   drive, not the process working directory. [`std::path::Path::is_relative`]
//!   does not catch it, because `C:foo` has a prefix but no root.
//!
//! - **Verbatim prefixes.** On Windows, [`std::fs::canonicalize`] returns
//!   *verbatim* paths: `\\?\C:\users\jeff` for drive paths and
//!   `\\?\UNC\server\share\…` for network shares. Those forms address the
//!   filesystem correctly, but they never compare equal to a user-built path,
//!   so they are rewritten to their classic equivalents.

use std::path::{Component, Path, PathBuf};

/// The classic Win32 path limit, in UTF-16 code units, including the
/// terminating NUL. Plain (non-verbatim) paths of 260 or more units cannot be
/// addressed by non-verbatim-aware APIs, so stripping the prefix there would
/// produce a path nothing can reopen.
#[cfg(windows)]
const MAX_CLASSIC_PATH: usize = 260;

/// The verbatim prefix Windows canonicalization produces.
#[cfg(windows)]
const VERBATIM_PREFIX: &str = r"\\?";

/// Marker following the verbatim prefix for network-share paths.
#[cfg(windows)]
const UNC_MARKER: &str = r"UNC\";

/// Rewrites the separators of `path` to the platform's own separator.
///
/// On Windows both `/` and `\` are path separators, so every forward slash
/// becomes a backslash. On Unix `/` is already the only separator and `\` is
/// an ordinary file-name character, so the path is returned unchanged.
pub(crate) fn normalize_separators(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        match path.to_str() {
            Some(text) => PathBuf::from(text.replace('/', "\\")),
            None => path.to_path_buf(),
        }
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Whether `path` carries a Windows drive prefix (`C:`, `C:foo`).
///
/// A drive-prefixed path is *drive-relative* on Windows: it resolves against
/// the working directory of that drive rather than the process working
/// directory, so joining one onto a base silently produces a path that points
/// somewhere else entirely.
///
/// The check is textual, so it behaves the same on every platform. On Unix,
/// `C:foo` is a legal file name — but it is never a meaningful relative path,
/// and a test that builds one has almost certainly made a mistake.
pub(crate) fn has_drive_prefix(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    let bytes = text.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Removes `.` segments and resolves `..` segments lexically.
///
/// The rules, applied per component of [`Path::components`]:
///
/// - `.` is dropped.
/// - `..` pops a preceding normal component; it is **dropped at a root**
///   (clamped — a root has no parent, so `/../x` normalizes to `/x`) and
///   **kept** when there is nothing to pop in a relative path (`../a` stays
///   `../a`).
/// - Root and Windows prefix components are preserved as-is.
///
/// This is purely lexical: no filesystem access, so symlinks are neither
/// resolved nor detected.
pub(crate) fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut components: Vec<Component<'_>> = Vec::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match components.last() {
                Some(Component::Normal(_)) => {
                    components.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => components.push(component),
            },
            _ => components.push(component),
        }
    }

    components
        .into_iter()
        .map(Component::as_os_str)
        .collect::<PathBuf>()
}

/// Strips the Windows `\\?\` verbatim prefix from a canonicalized path.
///
/// Transforms the two verbatim forms [`std::fs::canonicalize`] produces on
/// Windows into their classic equivalents:
///
/// - `\\?\C:\users\jeff` → `C:\users\jeff` (drive form)
/// - `\\?\UNC\server\share\file` → `\\server\share\file` (network form)
///
/// Three kinds of input are returned **unchanged**, because no safe plain
/// form exists:
///
/// - Paths whose plain form would be at or beyond the classic `MAX_PATH`
///   limit — only the verbatim prefix lets APIs address those, so stripping
///   would produce a path nothing can reopen.
/// - Verbatim *device* paths (`\\?\COM1`, `\\?\PIPE\…`): there is no drive or
///   share to expose, and stripping would fabricate a bogus relative path.
/// - Non-UTF-8-representable paths, which cannot be rebuilt safely here.
///
/// On non-Windows platforms this is the identity function.
///
/// # Example
///
/// ```
/// let stripped = marked_path::strip_verbatim_prefix(std::path::PathBuf::from("/etc/hosts"));
/// assert_eq!(stripped, std::path::PathBuf::from("/etc/hosts"));
/// ```
pub fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        strip_verbatim_windows(path)
    }
    #[cfg(not(windows))]
    {
        path
    }
}

/// Windows implementation of [`strip_verbatim_prefix`].
#[cfg(windows)]
fn strip_verbatim_windows(path: PathBuf) -> PathBuf {
    // Not representable as UTF-8: rebuilding the path would be guesswork, and
    // the verbatim form is accepted everywhere a plain path is.
    let Some(text) = path.to_str() else {
        return path;
    };
    let Some(rest) = text.strip_prefix(VERBATIM_PREFIX) else {
        return path;
    };

    let plain = match rest.strip_prefix(UNC_MARKER) {
        Some(share) => format!(r"\\{share}"),
        None if is_drive_path(rest) => rest.to_owned(),
        None => return path,
    };

    // A plain path at or beyond the classic limit is un-addressable without
    // the prefix; keep the verbatim form rather than emitting a broken path.
    if plain.chars().map(char::len_utf16).sum::<usize>() >= MAX_CLASSIC_PATH {
        return path;
    }
    PathBuf::from(plain)
}

/// Whether `rest` (a verbatim path minus its `\\?\` prefix) names a drive
/// root or something under one — `C:\`, `C:\users\…`. Anything else after the
/// prefix is a device path.
#[cfg(windows)]
fn is_drive_path(rest: &str) -> bool {
    matches!(
        rest.as_bytes(),
        [drive, b':', _sep @ (b'\\' | b'/'), ..] if drive.is_ascii_alphabetic()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[test]
    fn drive_prefix_is_detected_for_a_drive_relative_path() {
        // Given a Windows drive-relative path.
        let path = Path::new("C:foo");

        // When checking for a drive prefix.
        let detected = has_drive_prefix(path);

        // Then the drive prefix is found.
        assert!(detected);
    }

    #[rstest::rstest]
    #[test]
    fn drive_prefix_is_detected_for_a_bare_drive() {
        // Given a bare drive path.
        let path = Path::new("C:");

        // When checking for a drive prefix.
        let detected = has_drive_prefix(path);

        // Then the drive prefix is found.
        assert!(detected);
    }

    #[rstest::rstest]
    #[test]
    fn drive_prefix_is_not_detected_for_a_plain_relative_path() {
        // Given an ordinary relative path.
        let path = Path::new("sub/dir/file.txt");

        // When checking for a drive prefix.
        let detected = has_drive_prefix(path);

        // Then no drive prefix is found.
        assert!(!detected);
    }

    #[rstest::rstest]
    #[test]
    fn drive_prefix_is_not_detected_for_a_letter_followed_by_a_non_colon() {
        // Given a relative path whose second character is not a colon.
        let path = Path::new("a/b");

        // When checking for a drive prefix.
        let detected = has_drive_prefix(path);

        // Then no drive prefix is found.
        assert!(!detected);
    }

    #[rstest::rstest]
    #[test]
    fn separators_are_normalized_to_the_platform_separator() {
        // Given a relative path written with forward slashes.
        let path = Path::new("a/b/c.txt");

        // When normalizing its separators.
        let normalized = normalize_separators(path);

        // Then every component is joined with the platform separator.
        let expected = ["a", "b", "c.txt"].join(std::path::MAIN_SEPARATOR_STR);
        assert_eq!(normalized, PathBuf::from(expected));
    }

    #[cfg(windows)]
    #[rstest::rstest]
    #[test]
    fn forward_slashes_become_backslashes_on_windows() {
        // Given a relative path written with forward slashes.
        let path = Path::new("a/b/c.txt");

        // When normalizing its separators.
        let normalized = normalize_separators(path);

        // Then no forward slash survives.
        assert_eq!(normalized, PathBuf::from(r"a\b\c.txt"));
    }

    #[cfg(windows)]
    #[rstest::rstest]
    #[test]
    fn lexical_normalization_preserves_a_drive_prefix_and_resolves_within_it() {
        // Given a drive-relative path with a `..` segment.
        let path = Path::new(r"C:foo\..\bar");

        // When normalizing it lexically.
        let normalized = normalize_lexical_path(path);

        // Then the drive prefix is preserved and the `..` resolves within
        // the drive-relative path.
        assert_eq!(normalized, PathBuf::from(r"C:bar"));
    }

    #[rstest::rstest]
    #[test]
    fn lexical_normalization_clamps_parent_at_the_root() {
        // Given an absolute path climbing past its own root.
        let path = Path::new("/x/../../y");

        // When normalizing it lexically.
        let normalized = normalize_lexical_path(path);

        // Then the `..` beyond the root is dropped, not kept.
        assert_eq!(normalized, PathBuf::from("/y"));
    }
}
