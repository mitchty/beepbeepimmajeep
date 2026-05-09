//! Shared logic crate so in future I can get more cute by having the host crate
//! use this stuff and/or test it as a simulator maybe. For now its more to
//! cover the unit tests for code that doesn't directly depend upon the esp32
//! itself.
//!
//! This all needs to be no std compatible as well. The host crate has no
//! constraint like that though.
#![no_std]

use core::fmt;
use libm::{atan2f, sqrtf};

/// This is just a silly function ignore it.
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}

/// Roll and pitch angles in degrees derived from the complementary filter.
///
/// **Roll**: rotation around the X axis tilting left/right.
/// **Pitch**: rotation around the Y axis tilting forward/back.
/// **Yaw**: not tracked, a magnetometer or something more is needed to detect roll across Z axis (gravity) with any precision which we don't have.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orientation {
    /// Degrees, positive = right-side up.
    pub roll: f32,
    /// Degrees, positive = nose up.
    pub pitch: f32,
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "roll={:+.1}° pitch={:+.1}°", self.roll, self.pitch)
    }
}

/// Single-step complementary filter for roll and pitch estimation off an imu.
///
/// Blends gyroscope's fast, drift-prone integration with an accelerometer's
/// slow, noise-prone tilt measurement:
///
/// ```text
/// angle = α·(angle + ω·dt) + (1−α)·accel_angle
/// ```
///
/// Typical values: `alpha ≈ 0.98`, `dt` = loop period in seconds.
pub struct ComplementaryFilter {
    roll: f32,
    pitch: f32,
    alpha: f32,
    dt: f32,
}

impl ComplementaryFilter {
    /// Create a new filter starting at level such that roll = pitch = 0°
    ///
    /// * `alpha` = weight given to the gyro integration 0.0–1.0, typically 0.98.
    /// * `dt`    = loop period in seconds must match how often `update` is called.
    pub fn new(alpha: f32, dt: f32) -> Self {
        Self {
            roll: 0.0,
            pitch: 0.0,
            alpha,
            dt,
        }
    }

    /// Feed one sample and return the updated orientation.
    ///
    /// * `accel`    = (x, y, z) normalized accelerometer reading in g.
    /// * `gyro_dps` = (x, y, z) gyroscope reading in degrees per second.
    pub fn update(&mut self, accel: (f32, f32, f32), gyro_dps: (f32, f32, f32)) -> Orientation {
        let (ax, ay, az) = accel;
        let (gx, gy, _gz) = gyro_dps;

        // Tilt angles from gravity vector mostly only accurate when stationary.
        let accel_roll = atan2f(ay, az).to_degrees();
        let accel_pitch = atan2f(-ax, sqrtf(ay * ay + az * az)).to_degrees();

        // Complementary blend: gyro tracks fast motion, accel corrects drift.
        self.roll = self.alpha * (self.roll + gx * self.dt) + (1.0 - self.alpha) * accel_roll;
        self.pitch = self.alpha * (self.pitch + gy * self.dt) + (1.0 - self.alpha) * accel_pitch;

        Orientation {
            roll: self.roll,
            pitch: self.pitch,
        }
    }
}

// TODO: need more to simulate things moving, probably better to do that in the
// host crate as a simulation itself.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_adds() {
        assert_eq!(add(1, 2), 3);
    }

    #[test]
    fn filter_level_board_reads_zero() {
        // Accel pointing straight down (0g, 0g, 1g) with no rotation.
        let mut f = ComplementaryFilter::new(0.98, 0.02);
        let orient = f.update((0.0, 0.0, 1.0), (0.0, 0.0, 0.0));
        assert!(orient.roll.abs() < 0.01, "roll={}", orient.roll);
        assert!(orient.pitch.abs() < 0.01, "pitch={}", orient.pitch);
    }

    #[test]
    fn filter_roll_90_from_accel() {
        // Board rolled 90°: gravity points along Y axis.
        let mut f = ComplementaryFilter::new(0.0, 0.02);
        let orient = f.update((0.0, 1.0, 0.0), (0.0, 0.0, 0.0));
        assert!((orient.roll - 90.0).abs() < 0.1, "roll={}", orient.roll);
    }
}
