//! Shared pointer gestures. Backends supply primitives and timing data only.
use anyhow::Result;

pub(crate) trait DragDriver {
    fn move_to(&mut self, x: i32, y: i32) -> Result<()>;
    fn left_button(&mut self, pressed: bool) -> Result<()>;
    fn wait(&mut self, millis: u32) {
        crate::utils::sleep(millis);
    }
}

/// Preserve each validated backend's pacing without duplicating the gesture.
pub(crate) struct DragTiming {
    pub steps: u32,
    pub move_settle_ms: u32,
    pub press_settle_ms: u32,
}

pub(crate) fn drag(
    driver: &mut impl DragDriver,
    timing: DragTiming,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Result<()> {
    driver.move_to(x0, y0)?;
    if timing.move_settle_ms > 0 {
        driver.wait(timing.move_settle_ms);
    }
    // Always attempt release, including a failed press or an intermediate move.
    let result = (|| {
        driver.left_button(true)?;
        if timing.press_settle_ms > 0 {
            driver.wait(timing.press_settle_ms);
        }
        for step in 1..=timing.steps {
            let progress = f64::from(step) / f64::from(timing.steps);
            let eased = progress * progress * (3.0 - 2.0 * progress);
            driver.move_to(
                x0 + (f64::from(x1 - x0) * eased).round() as i32,
                y0 + (f64::from(y1 - y0) * eased).round() as i32,
            )?;
            driver.wait(40);
        }
        // Hold stationary before release so a page drag does not become a fling.
        driver.wait(350);
        Ok(())
    })();
    let release = driver.left_button(false);
    result.and(release)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Driver {
        held: bool,
        fail_move: bool,
        moves: Vec<(i32, i32)>,
        elapsed: u32,
    }
    impl DragDriver for Driver {
        fn move_to(&mut self, x: i32, y: i32) -> Result<()> {
            anyhow::ensure!(!(self.held && self.fail_move), "movement failed");
            self.moves.push((x, y));
            Ok(())
        }
        fn left_button(&mut self, pressed: bool) -> Result<()> {
            self.held = pressed;
            Ok(())
        }
        fn wait(&mut self, millis: u32) {
            self.elapsed += millis;
        }
    }
    #[test]
    fn drag_preserves_pacing_and_releases_after_movement_failure() {
        for (steps, settle) in [(30, 40), (12, 0)] {
            let profile = || DragTiming {
                steps,
                move_settle_ms: settle,
                press_settle_ms: settle,
            };
            let mut driver = Driver::default();
            drag(&mut driver, profile(), 10, 20, 500, 60).unwrap();
            assert_eq!(driver.moves.first(), Some(&(10, 20)));
            assert_eq!(driver.moves.last(), Some(&(500, 60)));
            assert_eq!(driver.moves.len(), steps as usize + 1);
            assert_eq!(driver.elapsed, settle * 2 + steps * 40 + 350);
            assert!(!driver.held);
            let mut failing = Driver {
                fail_move: true,
                ..Default::default()
            };
            assert!(drag(&mut failing, profile(), 10, 20, 500, 60).is_err());
            assert!(
                !failing.held,
                "input errors must not leave the drag button held"
            );
        }
    }
}
