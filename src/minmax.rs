use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Commute;

/// A commutative data structure for tracking minimum and maximum values.
///
/// This also stores the number of samples.
#[derive(Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct MinMax<T> {
    len: u64,
    min: Option<T>,
    max: Option<T>,
}

impl<T: PartialOrd + Clone> MinMax<T> {
    /// Create an empty state where min and max values do not exist.
    #[must_use]
    pub fn new() -> MinMax<T> {
        Default::default()
    }

    /// Add a sample to the data.
    #[inline]
    pub fn add(&mut self, sample: T) {
        self.len += 1;
        if self.min.as_ref().map_or(true, |v| &sample < v) {
            self.min = Some(sample.clone());
        }
        if self.max.as_ref().map_or(true, |v| &sample > v) {
            self.max = Some(sample);
        }
    }

    /// Returns the minimum of the data set.
    ///
    /// `None` is returned if and only if the number of samples is `0`.
    #[inline]
    #[must_use]
    pub const fn min(&self) -> Option<&T> {
        self.min.as_ref()
    }

    /// Returns the maximum of the data set.
    ///
    /// `None` is returned if and only if the number of samples is `0`.
    #[inline]
    #[must_use]
    pub const fn max(&self) -> Option<&T> {
        self.max.as_ref()
    }

    /// Returns the number of data points.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns true if there are no data points.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: PartialOrd> Commute for MinMax<T> {
    #[inline]
    fn merge(&mut self, v: MinMax<T>) {
        self.len += v.len;
        if self.min.is_none() || (v.min.is_some() && v.min < self.min) {
            self.min = v.min;
        }
        if self.max.is_none() || (v.max.is_some() && v.max > self.max) {
            self.max = v.max;
        }
    }
}

impl<T: PartialOrd> Default for MinMax<T> {
    #[inline]
    fn default() -> MinMax<T> {
        MinMax {
            len: 0,
            min: None,
            max: None,
        }
    }
}

#[cfg(debug_assertions)]
impl<T: fmt::Debug> fmt::Debug for MinMax<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.min, &self.max) {
            (Some(min), Some(max)) => {
                write!(f, "[{min:?}, {max:?}]")
            }
            (&None, &None) => write!(f, "N/A"),
            _ => unreachable!(),
        }
    }
}

impl<T: PartialOrd + Clone> FromIterator<T> for MinMax<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> MinMax<T> {
        let mut v = MinMax::new();
        v.extend(it);
        v
    }
}

impl<T: PartialOrd + Clone> Extend<T> for MinMax<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for sample in it {
            self.add(sample);
        }
    }
}

#[cfg(test)]
mod test {
    use super::MinMax;
    use crate::Commute;

    #[test]
    fn minmax() {
        let minmax: MinMax<u32> = vec![1u32, 4, 2, 3, 10].into_iter().collect();
        assert_eq!(minmax.min(), Some(&1u32));
        assert_eq!(minmax.max(), Some(&10u32));
    }

    #[test]
    fn minmax_empty() {
        let minmax: MinMax<u32> = MinMax::new();
        assert!(minmax.is_empty());
    }

    #[test]
    fn minmax_merge_empty() {
        let mut mx1: MinMax<u32> = vec![1, 4, 2, 3, 10].into_iter().collect();
        assert_eq!(mx1.min(), Some(&1u32));
        assert_eq!(mx1.max(), Some(&10u32));

        mx1.merge(MinMax::default());
        assert_eq!(mx1.min(), Some(&1u32));
        assert_eq!(mx1.max(), Some(&10u32));
    }
}
