//! Time-visibility rules. The shaders implement exactly these; keeping a Rust copy lets
//! us test them and lets the UI count what is on screen without asking the GPU.

/// What every shader receives about time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    /// The current year (fractional while playing).
    pub year: f32,
    /// Visibility window in years; 0 means "everything so far".
    pub window: f32,
    /// Years over which a new thread draws itself in; 0 draws instantly.
    pub reveal: f32,
}

impl Clock {
    #[must_use]
    pub fn at(year: f32) -> Self {
        Self { year, window: 0.0, reveal: 25.0 }
    }

    /// Years since something appeared at `year` (negative: not yet).
    #[must_use]
    pub fn age(self, year: f32) -> f32 {
        self.year - year
    }

    /// Opacity multiplier for something that appeared `age` years ago: 1 while fresh,
    /// fading over the last fifth of the window, 0 once outside it.
    #[must_use]
    pub fn fade(self, age: f32) -> f32 {
        if age < 0.0 {
            0.0
        } else if self.window <= 0.0 {
            1.0
        } else {
            1.0 - smoothstep(self.window * 0.8, self.window, age)
        }
    }

    /// How much of a thread is drawn, 0..=1, or `None` if it isn't visible at all.
    #[must_use]
    pub fn drawn(self, age: f32) -> Option<f32> {
        if age < 0.0 || self.fade(age) <= 0.0 {
            return None;
        }
        Some(if self.reveal > 0.0 { (age / self.reveal).min(1.0) } else { 1.0 })
    }

    /// Whether an item spanning `from..=until` is visible now.
    #[must_use]
    pub fn shows(self, from: f32, until: f32) -> bool {
        self.age(from) >= 0.0 && self.fade(self.age(until).max(0.0)) > 0.0
    }
}

/// GLSL/WGSL `smoothstep`.
#[must_use]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_before_its_year() {
        let c = Clock::at(1000.0);
        assert_eq!(c.drawn(c.age(1001.0)), None);
        assert!(!c.shows(1001.0, 1001.0));
    }

    #[test]
    fn threads_draw_in_then_stay() {
        let c = Clock::at(1000.0);
        assert_eq!(c.drawn(0.0), Some(0.0));
        assert!((c.drawn(12.5).unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(c.drawn(500.0), Some(1.0));
    }

    #[test]
    fn window_fades_out_old_items() {
        let c = Clock { window: 100.0, ..Clock::at(1000.0) };
        assert_eq!(c.fade(10.0), 1.0);
        assert!(c.fade(90.0) > 0.0 && c.fade(90.0) < 1.0);
        assert_eq!(c.fade(100.0), 0.0);
        assert_eq!(c.drawn(150.0), None);
    }

    #[test]
    fn spans_stay_visible_until_their_end_leaves_the_window() {
        let c = Clock { window: 50.0, ..Clock::at(2000.0) };
        assert!(c.shows(1900.0, 1980.0), "ended 20 years ago");
        assert!(!c.shows(1800.0, 1900.0), "ended 100 years ago");
    }
}
