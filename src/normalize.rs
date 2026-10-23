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
//!   filesystem correctly, but they never compare equal to a user-built
//!   path, so the verbatim *disk* form is rewritten to its classic
//!   equivalent whenever one exists.

use std::path::{Component, Path, PathBuf};

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
/// Transforms the verbatim *disk* form [`std::fs::canonicalize`] produces on
/// Windows into its classic equivalent:
///
/// - `\\?\C:\users\jeff` → `C:\users\jeff`
///
/// The rewrite is delegated to the [`dunce`](https://docs.rs/dunce) crate,
/// which keeps the verbatim prefix whenever no unambiguous classic form
/// exists:
///
/// - Verbatim forms other than the disk form — network shares
///   (`\\?\UNC\server\share\…`) and device paths (`\\?\COM1`) — have no
///   drive to expose, so they are returned unchanged.
/// - A plain path longer than the classic 260-unit limit (counted in both
///   bytes and UTF-16 code units) is un-addressable without the prefix, so
///   the verbatim form is kept.
/// - Verbatim paths carrying components a classic path cannot — reserved
///   DOS names (`COM1.txt`), names ending in `.` or a space, embedded
///   `.`/`..` segments — are kept verbatim.
/// - Non-UTF-8-representable paths cannot be rebuilt safely, so they are
///   kept verbatim.
///
/// Input without a verbatim prefix is returned unchanged. On non-Windows
/// platforms dunce is a no-op, so this is the identity function there.
///
/// # Example
///
/// ```
/// let stripped = marked_path::strip_verbatim_prefix(std::path::PathBuf::from("/etc/hosts"));
/// assert_eq!(stripped, std::path::PathBuf::from("/etc/hosts"));
/// ```
pub fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    dunce::simplified(&path).to_path_buf()
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

    #[rstest::rstest]
    #[test]
    fn strip_verbatim_prefix_is_the_identity_on_plain_unix_paths() {
        // Given a plain absolute path with nothing to strip.
        let path = PathBuf::from("/etc/hosts");

        // When stripping the verbatim prefix.
        let stripped = strip_verbatim_prefix(path.clone());

        // Then the path is returned unchanged.
        assert_eq!(stripped, path);
    }

    /// Contract pins for [`strip_verbatim_prefix`], asserted against dunce's
    /// documented behavior. These cannot execute in a Linux CI environment;
    /// they are written to be review-checkable against dunce 1.0.5's own
    /// `safe`/`strip` test suite, from which every expectation below is
    /// taken.
    #[cfg(windows)]
    mod windows_contract {
        use super::*;

        #[rstest::rstest]
        #[test]
        fn verbatim_drive_form_strips_to_the_classic_drive_path() {
            // Given a short verbatim drive path.
            let verbatim = PathBuf::from(r"\\?\C:\users\jeff");

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(verbatim);

            // Then it becomes the classic drive form.
            assert_eq!(stripped, PathBuf::from(r"C:\users\jeff"));
        }

        #[rstest::rstest]
        #[test]
        fn verbatim_unc_form_is_kept_verbatim() {
            // Given a verbatim network-share path.
            let verbatim = PathBuf::from(r"\\?\UNC\server\share\file.txt");

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(verbatim.clone());

            // Then it is returned unchanged: dunce simplifies only the disk
            // form, because a share has no drive to expose.
            assert_eq!(stripped, verbatim);
        }

        #[rstest::rstest]
        #[test]
        fn verbatim_device_path_is_kept_verbatim() {
            // Given a verbatim device path with no drive to expose.
            let device = PathBuf::from(r"\\.\C:\notdisk");

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(device.clone());

            // Then it is returned unchanged.
            assert_eq!(stripped, device);
        }

        #[rstest::rstest]
        #[test]
        fn verbatim_path_with_a_reserved_dos_name_is_kept_verbatim() {
            // Given a verbatim path whose component is a reserved DOS name.
            let verbatim = PathBuf::from(r"\\?\C:\COM4.txt");

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(verbatim.clone());

            // Then it is returned unchanged: the classic form would be
            // reinterpreted by DOS-name handling.
            assert_eq!(stripped, verbatim);
        }

        #[rstest::rstest]
        #[test]
        fn verbatim_path_over_the_classic_limit_is_kept_verbatim() {
            // Given a verbatim drive path whose full form exceeds the classic
            // 260-unit limit (the prefix plus a long tail).
            let tail = "®".repeat(160);
            let verbatim = PathBuf::from(format!(r"\\?\c:\{}\{}", tail, tail));

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(verbatim.clone());

            // Then it is returned unchanged: the plain form would be
            // un-addressable for non-verbatim-aware APIs.
            assert_eq!(stripped, verbatim);
        }

        #[rstest::rstest]
        #[test]
        fn verbatim_path_under_the_classic_limit_strips() {
            // Given a verbatim drive path whose full form fits the classic
            // limit (dunce's own boundary test uses a 160-unit tail).
            let tail = "®".repeat(160);
            let verbatim = PathBuf::from(format!(r"\\?\c:\{tail}"));

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(verbatim);

            // Then the classic form comes back.
            assert_eq!(stripped, PathBuf::from(format!(r"c:\{tail}")));
        }

        #[rstest::rstest]
        #[test]
        fn plain_path_is_returned_unchanged_on_windows() {
            // Given an already-plain Windows path.
            let plain = PathBuf::from(r"C:\users\jeff");

            // When stripping the verbatim prefix.
            let stripped = strip_verbatim_prefix(plain.clone());

            // Then it is returned as-is.
            assert_eq!(stripped, plain);
        }
    }
}
