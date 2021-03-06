// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! [`SizeRules`] type

use super::AxisInfo;
use crate::geom::Size;

/// Margin sizes
///
/// Used by the layout system for margins around child widgets. Margins may be
/// drawn in and handle events like any other widget area.
#[derive(Copy, Clone, Debug, Default)]
pub struct Margins {
    /// Size of top/left margin
    pub first: Size,
    /// Size of bottom/right margin
    pub last: Size,
    /// Size of inter-widget horizontal/vertical margins
    pub inter: Size,
}

impl Margins {
    /// Zero-sized margins
    pub const ZERO: Margins = Margins::uniform(0, 0);

    /// Margins with equal size on each edge, and on each axis.
    pub const fn uniform(edge: u32, inter: u32) -> Self {
        Margins {
            first: Size::uniform(edge),
            last: Size::uniform(edge),
            inter: Size::uniform(inter),
        }
    }

    /// Generate `SizeRules` from self
    ///
    /// Assumes zero-sized content (usually added separately).
    ///
    /// Requires the number of child columns and rows.
    pub fn size_rules(&self, axis_info: AxisInfo, columns: u32, rows: u32) -> SizeRules {
        SizeRules::fixed(if !axis_info.vertical {
            self.first.0 + self.last.0 + self.inter.0 * columns.saturating_sub(1)
        } else {
            self.first.1 + self.last.1 + self.inter.1 * rows.saturating_sub(1)
        })
    }
}

/// Widget sizing information
///
/// Return value of [`kas::Widget::size_rules`].
///
/// This struct conveys properties such as the minimum size and preferred size
/// of the widgets being queried.
#[derive(Copy, Clone, Debug, Default)]
pub struct SizeRules {
    // minimum size
    a: u32,
    // maximum size; b >= a
    b: u32,
}

impl SizeRules {
    /// Empty (zero size)
    pub const EMPTY: Self = SizeRules { a: 0, b: 0 };

    /// A fixed size
    #[inline]
    pub fn fixed(size: u32) -> Self {
        SizeRules { a: size, b: size }
    }

    /// A variable size with given `min`-imum and `pref`-erred values.
    ///
    /// Required: `pref >= min`.
    #[inline]
    pub fn variable(min: u32, pref: u32) -> Self {
        if min > pref {
            panic!("SizeRules::variable(min, pref): min > pref !");
        }
        SizeRules { a: min, b: pref }
    }

    /// Use the maximum size of `self` and `rhs`.
    #[inline]
    pub fn max(self, rhs: Self) -> SizeRules {
        SizeRules {
            a: self.a.max(rhs.a),
            b: self.b.max(rhs.b),
        }
    }

    /// Get the minimum size
    #[inline]
    pub fn min_size(self) -> u32 {
        self.a
    }

    /// Like `self = self.max(x - y)` but handling negative values correctly
    // TODO: switch to i32?
    pub fn set_at_least_op_sub(&mut self, x: Self, y: Self) {
        if x.a > y.a {
            self.a = self.a.max(x.a - y.a);
        }
        if x.b > y.b {
            self.b = self.b.max(x.b - y.b);
        }
        self.b = self.a.max(self.b);
    }

    /// Reduce the minimum size
    ///
    /// If `min` is greater than the current minimum size, this has no effect.
    #[inline]
    pub fn reduce_min_to(&mut self, min: u32) {
        self.a = self.a.min(min);
    }

    #[doc(hidden)]
    /// Solve a sequence of rules
    ///
    /// Given a sequence of width / height `rules` from children (including a
    /// final value which is the total) and a `target` size, find an appropriate
    /// size for each child width / height.
    // TODO (const generics):
    // fn solve_seq<const N: usize>(out: &mut [u32; N], rules: &[Self; N + 1], target: u32)
    pub fn solve_seq(out: &mut [u32], rules: &[Self], target: u32) {
        #[allow(non_snake_case)]
        let N = out.len();
        assert!(rules.len() == N + 1);
        if N == 0 {
            return;
        }

        if target >= rules[N].a {
            // At or over minimum: distribute extra relative to preferences.
            // TODO: perhaps this should not use the minimum except as a minimum?

            let target_rel = target - rules[N].a;
            let pref_rel = rules[N].b - rules[N].a;
            let mut sum = 0;

            if pref_rel > 0 {
                let x = target_rel as f64 / pref_rel as f64;

                for n in 0..N {
                    // This will round down:
                    let r = rules[n];
                    let size = r.a + (x * (r.b - r.a) as f64) as u32;
                    out[n] = size;
                    sum += size;
                }
            } else {
                // special case: pref_rel == 0
                let add = target_rel / N as u32;
                for n in 0..N {
                    let size = rules[n].a + add;
                    out[n] = size;
                    sum += size;
                }
            }

            // The above may round down, which may leave us a little short.
            assert!(sum <= target);
            let rem = target - sum;
            assert!(rem as usize <= N);
            // Distribute to first rem. sizes.
            for n in 0..(rem as usize) {
                out[n] += 1;
            }
        } else {
            // Under minimum: reduce maximum allowed size.
            let mut excess = rules[N].a - target;

            let mut largest = 0;
            let mut num_equal = 0;
            let mut next_largest = 0;
            for n in 0..N {
                let a = rules[n].a;
                out[n] = a;
                if a == largest {
                    num_equal += 1;
                } else if a > largest {
                    next_largest = largest;
                    largest = a;
                    num_equal = 1;
                } else if a > next_largest {
                    next_largest = a;
                }
            }

            while excess > 0 {
                let step = (excess / num_equal).min(largest - next_largest);
                if step == 0 {
                    for n in 0..N {
                        if out[n] == largest {
                            out[n] -= 1;
                            if excess == 0 {
                                break;
                            }
                            excess -= 1;
                        }
                    }
                    break;
                }

                let thresh = next_largest;
                let mut num_add = 0;
                next_largest = 0;
                for n in 0..N {
                    let a = out[n];
                    if a == largest {
                        out[n] = a - step;
                    } else if a == thresh {
                        num_add += 1;
                    } else if a > next_largest {
                        next_largest = a;
                    }
                }
                excess -= step * num_equal;

                largest -= step;
                num_equal += num_add;
            }
        }
    }
}

impl std::ops::Add<SizeRules> for SizeRules {
    type Output = Self;

    #[inline]
    fn add(self, rhs: SizeRules) -> Self::Output {
        SizeRules {
            a: self.a + rhs.a,
            b: self.b + rhs.b,
        }
    }
}

impl std::ops::Add<u32> for SizeRules {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u32) -> Self::Output {
        SizeRules {
            a: self.a + rhs,
            b: self.b + rhs,
        }
    }
}

impl std::ops::AddAssign for SizeRules {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            a: self.a + rhs.a,
            b: self.b + rhs.b,
        };
    }
}
