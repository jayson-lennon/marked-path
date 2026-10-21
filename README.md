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

### Serialization

`MarkedPathBuf` serializes as a two-field object: the path and the marker
that constructed it. On deserialization the marker is a label, not an
authority — the content is validated against the target type, so a relative
path never reconstructs as `MarkedPathBuf<Absolute>`, whatever the label
claims. The bare-string shape is accepted too, so path strings stored before
this marker existed load unchanged.

```rust
use marked_path::{Absolute, MarkedPathBuf, MarkedPathAccess};

let abs = MarkedPathBuf::<Absolute>::new("/home/user")?;
assert_eq!(
    serde_json::to_string(&abs)?,
    r#"{"path":"/home/user","marker":"absolute"}"#
);

// Round-trips through the tagged form.
let loaded: MarkedPathBuf<Absolute> = serde_json::from_str(&serde_json::to_string(&abs)?)?;
assert_eq!(loaded.as_path(), std::path::Path::new("/home/user"));

// Legacy bare strings still load, with the same content validation.
let legacy: MarkedPathBuf<Absolute> = serde_json::from_str(r#""/home/user""#)?;
assert_eq!(legacy.as_path(), std::path::Path::new("/home/user"));

// A relative string is rejected here, in both shapes.
assert!(serde_json::from_str::<MarkedPathBuf<Absolute>>(r#""documents""#).is_err());
assert!(serde_json::from_str::<MarkedPathBuf<Absolute>>(r#"{"path":"documents","marker":"absolute"}"#).is_err());
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Stripping a base and normalizing lexically

`strip_prefix` removes a base directory and returns the remainder as a
`MarkedPathBuf<Relative>`, so display-relative paths stay type-checked
instead of degrading to `PathBuf`. `normalize_lexical` resolves `.` and
`..` segments without touching the filesystem; at a root the `..` clamps,
and the marker is preserved.

```rust
use marked_path::{Absolute, MarkedPathBuf, MarkedPathAccess, Relative};

let abs = MarkedPathBuf::<Absolute>::new("/home/user/docs")?;

// The remainder of the strip is a typed relative path.
let rel: MarkedPathBuf<Relative> = abs.strip_prefix("/home/user")?;
assert_eq!(rel.as_path(), std::path::Path::new("docs"));

// Stripping a base the path does not start with is an error.
assert!(abs.strip_prefix("/elsewhere").is_err());

// Dot segments resolve lexically — no filesystem access.
let messy = MarkedPathBuf::<Absolute>::new("/a/b/../c")?;
assert_eq!(messy.normalize_lexical().as_path(), std::path::Path::new("/a/c"));
# Ok::<(), marked_path::PathError>(())
```

## License

[LGPL-3.0-or-later](https://www.gnu.org/licenses/lgpl-3.0.en.html)
