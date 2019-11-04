// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types

use std::fmt;
use std::num::NonZeroU32;
use std::u32;

use crate::{geom::Rect, Core};

/// Widget identifier
///
/// All widgets within a window are assigned a unique numeric identifier. This
/// type may be tested for equality and order.
///
/// Note: identifiers are first assigned when a window is instantiated by the
/// toolkit.
#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, PartialEq, Eq)]
pub struct WidgetId(NonZeroU32);

impl WidgetId {
    #[doc(hidden)]
    pub const FIRST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

    #[doc(hidden)]
    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU32::new(self.0.get() + 1).unwrap())
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        WidgetId::LAST
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

/// Common widget data
///
/// All widgets should embed a `core: CoreData` field in order to implement the
/// [`Core`] macro.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub id: WidgetId,
    pub rect: Rect,
}

impl Core for CoreData {
    #[inline]
    fn core_data(&self) -> &CoreData {
        self
    }

    #[inline]
    fn core_data_mut(&mut self) -> &mut CoreData {
        self
    }
}
