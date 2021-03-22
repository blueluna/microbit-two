#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};
pub use nrf52833_hal as hal;

pub mod images;
mod led_matrix;

use defmt_rtt as _; // global logger
use panic_probe as _;
// TODO(5) adjust HAL import
// use some_hal as _; // memory layout

defmt::timestamp! {"{=u64}", {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        // NOTE(no-CAS) `timestamps` runs with interrupts disabled
        let n = COUNT.load(Ordering::Relaxed);
        COUNT.store(n + 1, Ordering::Relaxed);
        n as u64
    }
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

pub use led_matrix::{Image, LedMatrix};
