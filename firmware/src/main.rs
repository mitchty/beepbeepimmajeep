//! Firmware entry-point for the ESP32-C3 DevKit-RUST-v2 only for now.
#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config, I2c},
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
};
use esp_println::println;
use icm42670::{prelude::*, Address, Icm42670};
// This is broken and so is defmt, so just using println! for now
//use log::{error, info};
use shared::ComplementaryFilter;

esp_bootloader_esp_idf::esp_app_desc!();

/// IMU sampling period must match the embassy timer interval to work sanely.
const DT: f32 = 0.02; // 50 Hz
const DT_MS: u64 = (DT * 1000.0) as u64;

/// Complementary filter tuning: weight given to gyro vs accelerometer, value
/// chosen out of my butt hasn't been tuned/validated yet "seems to work here
/// hold my beer" style.
const ALPHA: f32 = 0.98;

/// Print every N ticks, N x DT_MS = periodic console print interval.
/// 25 x 20 ms = 500 ms
const PRINT_EVERY: u32 = 25;

#[esp_rtos::main]
async fn main(_spawner: embassy_executor::Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialize esp-rtos scheduler for embassy-time to work.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_ints = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    // ICM-42670-P on i2c bus is at SDA=GPIO7 and SCL=GPIO8 on the
    // DevKit-RUST-2, the v1 has diff layout on i2c for both.
    let i2c = match I2c::new(peripherals.I2C0, Config::default()) {
        Ok(bus) => bus.with_sda(peripherals.GPIO7).with_scl(peripherals.GPIO8),
        Err(e) => {
            println!("I2C init failed: {:?}", e);
            loop {}
        }
    };

    let mut imu = match Icm42670::new(i2c, Address::Primary) {
        Ok(imu) => imu,
        Err(e) => {
            println!("IMU init failed: {:?}", e);
            loop {}
        }
    };

    println!("IMU initialized filter loop = {} Hz", 1000 / DT_MS);

    let mut filter = ComplementaryFilter::new(ALPHA, DT);
    let mut tick: u32 = 0;

    loop {
        match (imu.accel_norm(), imu.gyro_norm()) {
            (Ok(accel), Ok(gyro)) => {
                let orientation = filter.update(
                    (accel.x, accel.y, accel.z),
                    (gyro.x, gyro.y, gyro.z),
                );

                tick += 1;
                if tick >= PRINT_EVERY {
                    tick = 0;
                    println!("{}", orientation);
                }
            }
            (Err(e), _) => println!("accel read: {:?}", e),
            (_, Err(e)) => println!("gyro read: {:?}", e),
        }

        Timer::after(Duration::from_millis(DT_MS)).await;
    }
}
