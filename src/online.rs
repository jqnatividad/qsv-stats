use std::fmt;

use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::Commute;

/// Compute the standard deviation of a stream in constant space.
pub fn stddev<'a, I, T>(x: I) -> f64
where
    I: IntoIterator<Item = T>,
    T: Into<&'a f64>,
{
    let it = x.into_iter();
    stddev(it)
}

/// Compute the variance of a stream in constant space.
pub fn variance<'a, I, T>(x: I) -> f64
where
    I: IntoIterator<Item = T>,
    T: Into<&'a f64>,
{
    let it = x.into_iter();
    variance(it)
}

/// Compute the mean of a stream in constant space.
pub fn mean<'a, I, T>(x: I) -> f64
where
    I: IntoIterator<Item = T>,
    T: Into<&'a f64>,
{
    let it = x.into_iter();
    mean(it)
}

/// Online state for computing mean, variance and standard deviation.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct OnlineStats {
    size: u64,
    mean: f64,
    q: f64,
    harmonic_sum: f64,
    geometric_sum: f64,
    has_zero: bool,
    has_negative: bool,
}

impl OnlineStats {
    /// Create initial state.
    ///
    /// Population size, variance and mean are set to `0`.
    #[must_use]
    pub fn new() -> OnlineStats {
        Default::default()
    }

    /// Initializes `OnlineStats` from a sample.
    #[must_use]
    pub fn from_slice<T: ToPrimitive>(samples: &[T]) -> OnlineStats {
        samples.iter().map(|n| n.to_f64().unwrap()).collect()
    }

    /// Return the current mean.
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.is_empty() {
            f64::NAN
        } else {
            self.mean
        }
    }

    /// Return the current standard deviation.
    #[must_use]
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Return the current variance.
    // https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance
    #[must_use]
    pub fn variance(&self) -> f64 {
        self.q / (self.size as f64)
    }

    /// Return the current harmonic mean.
    #[must_use]
    pub fn harmonic_mean(&self) -> f64 {
        if self.is_empty() || self.has_zero || self.has_negative {
            f64::NAN
        } else {
            (self.size as f64) / self.harmonic_sum
        }
    }

    /// Return the current geometric mean.
    #[must_use]
    pub fn geometric_mean(&self) -> f64 {
        if self.is_empty() {
            f64::NAN
        } else if self.has_zero {
            0.0
        } else if self.has_negative
            || self.geometric_sum.is_infinite()
            || self.geometric_sum.is_nan()
        {
            f64::NAN
        } else {
            (self.geometric_sum / (self.size as f64)).exp()
        }
    }

    // TODO: Calculate kurtosis
    // also see https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance

    /// Add a new sample.
    #[inline]
    pub fn add<T: ToPrimitive>(&mut self, sample: &T) {
        let sample = sample.to_f64().unwrap();
        // Taken from: https://en.wikipedia.org/wiki/Standard_deviation#Rapid_calculation_methods
        // See also: https://api.semanticscholar.org/CorpusID:120126049
        let oldmean = self.mean;
        self.size += 1;
        let delta = sample - oldmean;
        self.mean += delta / (self.size as f64);
        let delta2 = sample - self.mean;
        self.q += delta * delta2;

        // Update harmonic mean sum (avoid division by zero)
        if sample != 0.0 {
            self.harmonic_sum += 1.0 / sample;
        }

        // Update geometric mean tracking
        if sample == 0.0 {
            self.has_zero = true;
        } else if sample < 0.0 {
            self.has_negative = true;
        } else if sample > 0.0 {
            self.geometric_sum += sample.ln();
        }
    }

    /// Add a new NULL value to the population.
    ///
    /// This increases the population size by `1`.
    #[inline]
    pub fn add_null(&mut self) {
        self.add(&0usize);
    }

    /// Returns the number of data points.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.size as usize
    }

    /// Returns if empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        #[cfg(feature = "nightly")]
        {
            if std::intrinsics::likely(self.size != 0) {
                false
            } else {
                true
            }
        }
        #[cfg(not(feature = "nightly"))]
        {
            self.size == 0
        }
    }
}

impl Commute for OnlineStats {
    #[inline]
    fn merge(&mut self, v: OnlineStats) {
        // Taken from: https://en.wikipedia.org/wiki/Standard_deviation#Combining_standard_deviations
        let (s1, s2) = (self.size as f64, v.size as f64);
        let meandiffsq = (self.mean - v.mean) * (self.mean - v.mean);

        self.size += v.size;

        //self.mean = ((s1 * self.mean) + (s2 * v.mean)) / (s1 + s2);
        // below is the fused multiply add version of the statement above
        // its more performant as we're taking advantage of a CPU instruction
        self.mean = s1.mul_add(self.mean, s2 * v.mean) / (s1 + s2);

        // self.q += v.q + meandiffsq * s1 * s2 / (s1 + s2);
        // below is the fused multiply add version of the statement above
        self.q += v.q + f64::mul_add(meandiffsq, s1 * s2 / (s1 + s2), 0.0);

        self.harmonic_sum += v.harmonic_sum;
        self.geometric_sum += v.geometric_sum;
        self.has_negative |= v.has_negative;
    }
}

impl Default for OnlineStats {
    fn default() -> OnlineStats {
        OnlineStats {
            size: 0,
            mean: 0.0,
            q: 0.0,
            harmonic_sum: 0.0,
            geometric_sum: 0.0,
            has_zero: false,
            has_negative: false,
        }
    }
}

impl fmt::Debug for OnlineStats {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.10} +/- {:.10}", self.mean(), self.stddev())
    }
}

impl<T: ToPrimitive> FromIterator<T> for OnlineStats {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> OnlineStats {
        let mut v = OnlineStats::new();
        v.extend(it);
        v
    }
}

impl<T: ToPrimitive> Extend<T> for OnlineStats {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for sample in it {
            self.add(&sample);
        }
    }
}

#[cfg(test)]
mod test {
    use super::OnlineStats;
    use {crate::merge_all, crate::Commute};

    #[test]
    fn online() {
        // TODO: Convert this to a quickcheck test.
        let expected = OnlineStats::from_slice(&[1usize, 2, 3, 2, 4, 6]);

        let var1 = OnlineStats::from_slice(&[1usize, 2, 3]);
        let var2 = OnlineStats::from_slice(&[2usize, 4, 6]);
        let mut got = var1;
        got.merge(var2);
        assert_eq!(expected.stddev(), got.stddev());
        assert_eq!(expected.mean(), got.mean());
        assert_eq!(expected.variance(), got.variance());
    }

    #[test]
    fn online_empty() {
        let expected = OnlineStats::new();
        assert!(expected.is_empty());
    }

    #[test]
    fn online_many() {
        // TODO: Convert this to a quickcheck test.
        let expected = OnlineStats::from_slice(&[1usize, 2, 3, 2, 4, 6, 3, 6, 9]);

        let vars = vec![
            OnlineStats::from_slice(&[1usize, 2, 3]),
            OnlineStats::from_slice(&[2usize, 4, 6]),
            OnlineStats::from_slice(&[3usize, 6, 9]),
        ];
        assert_eq!(
            expected.stddev(),
            merge_all(vars.clone().into_iter()).unwrap().stddev()
        );
        assert_eq!(
            expected.mean(),
            merge_all(vars.clone().into_iter()).unwrap().mean()
        );
        assert_eq!(
            expected.variance(),
            merge_all(vars.into_iter()).unwrap().variance()
        );
    }

    #[test]
    fn test_means() {
        let mut stats = OnlineStats::new();
        stats.extend(vec![2.0f64, 4.0, 8.0]);

        // Arithmetic mean = (2 + 4 + 8) / 3 = 4.666...
        assert!((stats.mean() - 4.666666666667).abs() < 1e-10);

        // Harmonic mean = 3 / (1/2 + 1/4 + 1/8) = 3.428571429
        assert_eq!("3.42857143", format!("{:.8}", stats.harmonic_mean()));

        // Geometric mean = (2 * 4 * 8)^(1/3) = 4.0
        assert!((stats.geometric_mean() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_means_with_negative() {
        let mut stats = OnlineStats::new();
        stats.extend(vec![-2.0f64, 2.0]);

        // Arithmetic mean = (-2 + 2) / 2 = 0
        assert!(stats.mean().abs() < 1e-10);

        // Geometric mean is NaN for negative numbers
        assert!(stats.geometric_mean().is_nan());

        // Harmonic mean is undefined when values have different signs
        assert!(stats.harmonic_mean().is_nan());
    }

    #[test]
    fn test_means_with_zero() {
        let mut stats = OnlineStats::new();
        stats.extend(vec![0.0f64, 4.0, 8.0]);

        // Arithmetic mean = (0 + 4 + 8) / 3 = 4
        assert!((stats.mean() - 4.0).abs() < 1e-10);

        // Geometric mean = (0 * 4 * 8)^(1/3) = 0
        assert!(stats.geometric_mean().abs() < 1e-10);

        // Harmonic mean is undefined when any value is 0
        assert!(stats.harmonic_mean().is_nan());
    }

    #[test]
    fn test_means_single_value() {
        let mut stats = OnlineStats::new();
        stats.extend(vec![5.0f64]);

        // All means should equal the single value
        assert!((stats.mean() - 5.0).abs() < 1e-10);
        assert!((stats.geometric_mean() - 5.0).abs() < 1e-10);
        assert!((stats.harmonic_mean() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_means_empty() {
        let stats = OnlineStats::new();

        // All means should be NaN for empty stats
        assert!(stats.mean().is_nan());
        assert!(stats.geometric_mean().is_nan());
        assert!(stats.harmonic_mean().is_nan());
    }
}
