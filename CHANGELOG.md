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
