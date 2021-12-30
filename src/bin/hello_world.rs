#![no_main]
#![no_std]

use microbit_two;
use microbit_two::hal;
use microbit_two::hal::pac;
use rtic::app;

#[app(device = pac, peripherals = true)]
mod app {
    use super::{hal, pac};
    use hal::{clocks, timer::Instance};
    use pac::{RTC0, TIMER0};

    #[local]
    struct LocalResources {
        rtc_0: hal::rtc::Rtc<RTC0>,
        timer_0: TIMER0,
    }

    #[shared]
    struct SharedResources {}

    #[init]
    fn init(cx: init::Context) -> (SharedResources, LocalResources, init::Monotonics) {
        // Configure to use external clocks, and start them
        let _clocks = clocks::Clocks::new(cx.device.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_synth()
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

        (
            SharedResources {},
            LocalResources {
                rtc_0,
                timer_0: cx.device.TIMER0,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIMER0, local = [timer_0])]
    fn timer(cx: timer::Context) {
        cx.local.timer_0.timer_reset_event();
        defmt::info!("~ timer ~");
    }

    #[task(binds = RTC0, local = [rtc_0])]
    fn rtc(cx: rtc::Context) {
        let _ = cx
            .local
            .rtc_0
            .is_event_triggered(hal::rtc::RtcInterrupt::Tick);
        defmt::info!("~ RTC ~");
    }
}
