## [0.3.0] - 2026-10-20

### 🐛 Bug Fixes

- Normalize path separators to the platform's own separator on construction, so a
  relative path written with forward slashes no longer leaves a mixed-separator path
  behind on Windows
- Reject Windows drive-prefixed paths (`C:`, `C:foo`) in `Relative` on every platform.
  A drive-relative path resolves against the current drive's working directory
  rather than the process working directory, and `Path::is_relative` does not
  catch it because such a path has a prefix but no root
- Strip the Windows `\\?\` verbatim prefix before the canonical-form check, so
  `CanonicalPath` accepts a path that came out of `std::fs::canonicalize` and
  compares equal to a user-built plain path

### 🚀 Features

- Implement `Deref<Target = Path>` for `MarkedPathBuf` and `MarkedPath`, so the
  marked types can be used wherever a path is expected without an explicit
  conversion

### ⚙️ Miscellaneous Tasks

- Fix the repository URL typo and exclude the mutation-testing output directories
  from the published package

## [0.2.1] - 2026-04-17

### 🚀 Features

- Add join_relative to MarkedPathBuf<Absolute> and MarkedPathBuf<Relative>
## [0.2.0] - 2026-03-29

### 🚜 Refactor

- Extract delegated path methods into MarkedPathAccess trait

### 📚 Documentation

- Link to license

### 🧪 Testing

- Add coverage for From<MarkedPathBuf<M>> for PathBuf conversion

### ⚙️ Miscellaneous Tasks

- Fix clippy pedantic lints (elidable_lifetime_names, uninlined_format_args)
- Bump version
- Add release toml for cargo release command
- Add changelog
- Release marked-path version 0.2.0
## [0.1.0] - 2026-03-28

### 🚀 Features

- Remove error-stack
- Add justfile
- Add PathBuf/Path method wrappers to MarkedPath

### 🐛 Bug Fixes

- Mutant tests
- Remove CanonicalPath::push_path to preserve canonical invariant
- Rename method
- Make into_marked public
- Change MSRV
- Prevent set_file_name/with_file_name from breaking Relative path invariant
- Change license to LGPL-v3
- README updated with latest code

### 🚜 Refactor

- Split types into dedicated modules
- Genericize PathBuf parameters with Into<PathBuf>
- Genericize from_path with AsRef<Path> bound
- Split MarkedPath into borrowed MarkedPath and owned MarkedPathBuf

### 📚 Documentation

- Add crate-level documentation to lib.rs
- Add doc comments to all new MarkedPath methods

### 🎨 Styling

- Use where clauses for generic bounds on new constructors

### 🧪 Testing

- Verify empty string is accepted as a valid relative path

### ⚙️ Miscellaneous Tasks

- Update clippy config + prep for release
- Clippy lints
