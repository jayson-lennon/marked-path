# marked-path

Type-safe path wrappers with compile-time guarantees about whether a path is
absolute or relative.

## Overview

`marked-path` provides three core types:

- **`MarkedPath<Absolute>`** — A path that is guaranteed to be absolute at
  construction time.
- **`MarkedPath<Relative>`** — A path that is guaranteed to be relative at
  construction time.
- **`CanonicalPath`** — An absolute path that has been fully resolved (no `.`
  or `..` components, all symlinks followed), guaranteeing the path existed at
  construction time.

These types use Rust's type system (via phantom marker types) to prevent
accidentally mixing absolute and relative paths at compile time.

## Example

```rust
use std::path::PathBuf;
use marked_path::{MarkedPath, Absolute, Relative};

// Create an absolute path (validated at construction)
let abs = MarkedPath::<Absolute>::new(PathBuf::from("/home/user"))?;

// Create a relative path
let rel = MarkedPath::<Relative>::new(PathBuf::from("documents/file.txt"))?;

// You can push relative paths onto absolute paths
let mut abs = MarkedPath::<Absolute>::new(PathBuf::from("/home"))?;
abs.push_path(&rel);
# Ok::<(), marked_path::PathError>(())
```

## License

AGPL-3.0-or-later
