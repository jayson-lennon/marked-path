# marked-path

[![Crates.io](https://img.shields.io/crates/v/marked-path.svg)](https://crates.io/crates/marked-path)
[![License: LGPL-3.0-or-later](https://img.shields.io/badge/License-LGPL--3.0--or--later-blue.svg)](https://opensource.org/license/lgpl-3-0)
[![Repository](https://img.shields.io/badge/repository-GitHub-black)](https://github.com/jayson-lennnon/marked-path)

Type-safe path wrappers with compile-time guarantees about whether a path is
absolute or relative.

## Overview

`marked-path` provides four core types:

- **`MarkedPathBuf<M>`** — An owned path wrapper (like `PathBuf`) that
  guarantees the path is [`Absolute`] or [`Relative`] at construction time.
- **`MarkedPath<'a, M>`** — A borrowed path wrapper (like `Path`), obtained
  via `.as_marked_path()`.
- **`CanonicalPath`** — An absolute path that has been fully resolved (no `.`
  or `..` components, all symlinks followed), guaranteeing the path existed at
  construction time.
- **`PathError`** — Error type returned when a path operation fails.

These types use Rust's type system (via phantom marker types) to prevent
accidentally mixing absolute and relative paths at compile time.

## Example

```rust
use marked_path::{Absolute, MarkedPathBuf, Relative};

// Create an absolute path (validated at construction)
let abs = MarkedPathBuf::<Absolute>::new("/home/user")?;

// Create a relative path
let rel = MarkedPathBuf::<Relative>::new("documents/file.txt")?;

// Push a relative path onto an absolute path
let mut abs = MarkedPathBuf::<Absolute>::new("/home")?;
abs.push(&rel.as_marked_path());
assert_eq!(abs.as_path(), std::path::Path::new("/home/documents/file.txt"));
# Ok::<(), marked_path::PathError>(())
```

## More examples

### Joining typed paths

```rust
use marked_path::{Absolute, MarkedPathBuf, Relative};

let abs = MarkedPathBuf::<Absolute>::new("/home/user")?;
let rel = MarkedPathBuf::<Relative>::new("documents/file.txt")?;

// Infallible join between typed paths
let combined = abs.as_marked_path().join_relative(&rel.as_marked_path());
assert_eq!(combined.as_path(), std::path::Path::new("/home/user/documents/file.txt"));
# Ok::<(), marked_path::PathError>(())
```

### Canonicalization

```rust
use marked_path::{CanonicalPath, MarkedPathBuf, Absolute};

let abs = MarkedPathBuf::<Absolute>::new("/etc/hosts")?;

// Canonicalize an absolute path (resolves symlinks, ., ..)
let canonical = abs.as_marked_path().canonicalize()?;

// Or create directly from any path reference
let canonical = CanonicalPath::from_path("/etc/hosts")?;

// Convert back to a MarkedPathBuf<Absolute>
let abs: MarkedPathBuf<Absolute> = canonical.into_marked();
# Ok::<(), marked_path::PathError>(())
```

### Parsing from strings

```rust
use std::str::FromStr;
use marked_path::{Absolute, MarkedPathBuf, Relative};

let abs: MarkedPathBuf<Absolute> = "/home/user".parse()?;
let rel: MarkedPathBuf<Relative> = "documents/file.txt".parse()?;

// Convert to a plain PathBuf
let pathbuf: std::path::PathBuf = abs.into();
# Ok::<(), marked_path::PathError>(())
```

## License

[LGPL-3.0-or-later](https://www.gnu.org/licenses/lgpl-3.0.en.html)
