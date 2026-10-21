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

//! The sealed marker contract behind the wire and `Debug` impls.
//!
//! [`Marker`] is implemented only by the crate's two marker types,
//! [`Absolute`](crate::Absolute) and [`Relative`](crate::Relative). That is
//! what lets [`MarkedPathBuf`](crate::MarkedPathBuf) offer blanket
//! `Serialize`/`Deserialize`/`Debug` impls that can name the marker and
//! validate untrusted content against it, without those impls being
//! extendable by outside marker types.

use std::path::Path;

use crate::absolute::Absolute;
use crate::marked_path::PathError;
use crate::relative::{Relative, is_relative_path};

mod sealed {
    use super::{Absolute, Relative};

    pub trait Sealed {}
    impl Sealed for Absolute {}
    impl Sealed for Relative {}
}

pub(crate) use sealed::Sealed;

/// Marker behavior shared by the crate's zero-sized marker types.
///
/// Gives the generic impls on [`MarkedPathBuf`](crate::MarkedPathBuf) a name
/// for the serialized form and a validation rule for untrusted input.
pub(crate) trait Marker: Sealed {
    /// The marker's name in the serialized form (`absolute` / `relative`).
    const NAME: &'static str;

    /// The marker's name as shown by `Debug` (`Absolute` / `Relative`).
    const LABEL: &'static str;

    /// Validates that `path` satisfies this marker's invariant.
    fn validate(path: &Path) -> Result<(), PathError>;
}

impl Marker for Absolute {
    const NAME: &'static str = "absolute";
    const LABEL: &'static str = "Absolute";

    fn validate(path: &Path) -> Result<(), PathError> {
        if path.is_absolute() {
            Ok(())
        } else {
            Err(PathError::NotAbsolute)
        }
    }
}

impl Marker for Relative {
    const NAME: &'static str = "relative";
    const LABEL: &'static str = "Relative";

    fn validate(path: &Path) -> Result<(), PathError> {
        // Same rule as `MarkedPathBuf<Relative>::new`: not absolute, and no
        // Windows drive prefix (`C:foo` is drive-relative, resolving against
        // the working directory of that drive rather than the process
        // working directory).
        if is_relative_path(path) {
            Ok(())
        } else {
            Err(PathError::NotRelative)
        }
    }
}
