#![no_main]
#![no_std]

use microbit_two as _;

use rtic::app;

use crate::hal::pac;
use nrf52833_hal as hal;

use hal::{clocks, timer::Instance};
use pac::{RTC0, TIMER0};

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        rtc_0: hal::rtc::Rtc<RTC0>,
        timer_0: TIMER0,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        // Configure to use external clocks, and start them
        let _clocks = clocks::Clocks::new(cx.device.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_external(clocks::LfOscConfiguration::NoExternalNoBypass)
            .start_lfclk();

        cx.device.TIMER0.set_periodic();
        cx.device.TIMER0.enable_interrupt();
        cx.device.TIMER0.timer_start(1_000_000u32);

        let mut rtc_0 = match hal::rtc::Rtc::new(cx.device.RTC0, 4095) {
            Ok(r) => r,
            Err(_) => unreachable!(),
        };
        rtc_0.enable_event(hal::rtc::RtcInterrupt::Tick);
        rtc_0.enable_interrupt(hal::rtc::RtcInterrupt::Tick, None);
        rtc_0.enable_counter();

        defmt::info!("~ initialisation ~");

        init::LateResources {
            timer_0: cx.device.TIMER0,
            rtc_0,
        }
    }

    #[task(binds = TIMER0, resources = [timer_0])]
    fn timer(cx: timer::Context) {
        cx.resources.timer_0.timer_reset_event();
        defmt::info!("~ timer ~");
    }

    #[task(binds = RTC0, resources = [rtc_0])]
    fn rtc(cx: rtc::Context) {
        let _ = cx
            .resources
            .rtc_0
            .is_event_triggered(hal::rtc::RtcInterrupt::Tick);
        defmt::info!("~ RTC ~");
    }
};
