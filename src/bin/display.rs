#![no_main]
#![no_std]

use microbit_two::hal;
use microbit_two::hal::pac;
use rtic::app;

#[app(device = pac, peripherals = true)]
mod app {
    use super::{hal, pac};

    use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
    use hal::{
        clocks,
        gpio::{self, Output, PushPull},
        timer::Instance,
    };
    use microbit_two::{lpm013m126a, spim};
    use pac::{RTC0, TIMER0, TIMER1, TIMER2};

    #[local]
    struct Local {
        rtc_0: hal::rtc::Rtc<RTC0>,
        timer_0: TIMER0,
        timer_1: TIMER1,
        timer_2: TIMER2,
        led_matrix: microbit_two::LedMatrix,
        jdi_com: hal::gpio::Pin<hal::gpio::Output<hal::gpio::PushPull>>,
        colour: u8,
    }

    #[shared]
    struct Shared {
        #[lock_free]
        jdi: lpm013m126a::Lpm013m126a<pac::SPIM3, hal::gpio::p0::P0_03<Output<PushPull>>>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        // Configure to use external clocks, and start them
        let _clocks = clocks::Clocks::new(cx.device.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_synth()
            .start_lfclk();

        let port0 = gpio::p0::Parts::new(cx.device.P0);
        let port1 = gpio::p1::Parts::new(cx.device.P1);

        cx.device.TIMER0.set_periodic();
        cx.device.TIMER0.enable_interrupt();
        cx.device.TIMER0.timer_start(160_u32);

        cx.device.TIMER1.set_periodic();
        cx.device.TIMER1.enable_interrupt();
        cx.device.TIMER1.timer_start(1_000_000_u32);

        cx.device.TIMER2.set_periodic();
        cx.device.TIMER2.enable_interrupt();
        cx.device.TIMER2.timer_start(41_666_u32);

        let mut rtc_0 = match hal::rtc::Rtc::new(cx.device.RTC0, 4095) {
            Ok(r) => r,
            Err(_) => unreachable!(),
        };
        rtc_0.enable_event(hal::rtc::RtcInterrupt::Tick);
        rtc_0.enable_interrupt(hal::rtc::RtcInterrupt::Tick, None);
        rtc_0.enable_counter();

        let mut led_matrix = microbit_two::LedMatrix::new(
            port0.p0_28.into_push_pull_output(gpio::Level::Low),
            port0.p0_11.into_push_pull_output(gpio::Level::Low),
            port0.p0_31.into_push_pull_output(gpio::Level::Low),
            port1.p1_05.into_push_pull_output(gpio::Level::Low),
            port0.p0_30.into_push_pull_output(gpio::Level::Low),
            port0.p0_21.into_push_pull_output(gpio::Level::Low),
            port0.p0_22.into_push_pull_output(gpio::Level::Low),
            port0.p0_15.into_push_pull_output(gpio::Level::Low),
            port0.p0_24.into_push_pull_output(gpio::Level::Low),
            port0.p0_19.into_push_pull_output(gpio::Level::Low),
        );

        led_matrix.display(microbit_two::images::SCALES);

        let jdi_spi = spim::Spim::new(
            cx.device.SPIM3,
            spim::Pins {
                sck: port0
                    .p0_17
                    .into_push_pull_output(gpio::Level::High)
                    .degrade(),
                mosi: Some(
                    port0
                        .p0_13
                        .into_push_pull_output(gpio::Level::High)
                        .degrade(),
                ),
                miso: None,
                csn: Some(
                    port0
                        .p0_02
                        .into_push_pull_output(gpio::Level::Low)
                        .degrade(),
                ),
                csn_pol: true,
            },
            hal::spim::Frequency::M4,
            hal::spim::MODE_0,
            0,
        );
        let mut jdi = lpm013m126a::Lpm013m126a::new(
            jdi_spi,
            port0.p0_03.into_push_pull_output(gpio::Level::Low),
        );

        let jdi_com = port1
            .p1_02
            .into_push_pull_output(gpio::Level::Low)
            .degrade();

        match jdi.init() {
            Err(_) => defmt::error!("Failed to initialize JDI"),
            Ok(_) => (),
        }

        for y in 0..lpm013m126a::DISPLAY_HEIGHT as u8 {
            for x in 0..lpm013m126a::DISPLAY_WIDTH as u8 {
                jdi.set_pixel(x, y, lpm013m126a::Palette8::White);
            }
        }

        defmt::info!("Initialized");

        let shared = Shared { jdi };
        let local = Local {
            timer_0: cx.device.TIMER0,
            timer_1: cx.device.TIMER1,
            timer_2: cx.device.TIMER2,
            rtc_0,
            led_matrix,
            jdi_com,
            colour: 0,
        };
        (shared, local, init::Monotonics())
    }

    #[task(binds = TIMER0, local = [timer_0, led_matrix])]
    fn timer0(cx: timer0::Context) {
        cx.local.timer_0.timer_reset_event();
        cx.local.led_matrix.update();
    }

    #[task(binds = TIMER1, local = [timer_1, jdi_com])]
    fn timer1(cx: timer1::Context) {
        cx.local.timer_1.timer_reset_event();
        let high = match cx.local.jdi_com.is_set_high() {
            Ok(s) => s,
            Err(_) => false,
        };
        if high {
            let _ = cx.local.jdi_com.set_low();
        } else {
            let _ = cx.local.jdi_com.set_high();
        }
    }

    #[task(binds = TIMER2, local = [timer_2, colour], shared = [jdi])]
    fn timer2(cx: timer2::Context) {
        cx.local.timer_2.timer_reset_event();
        *cx.local.colour += 1;
        if *cx.local.colour > 7 {
            *cx.local.colour = 0;
        }
        let c = microbit_two::lpm013m126a::Palette8::from(*cx.local.colour);
        let y = (lpm013m126a::lpm013m126a::DISPLAY_HEIGHT / 2) as u8;
        for x in 0..lpm013m126a::lpm013m126a::DISPLAY_WIDTH as u8 {
            cx.shared.jdi.set_pixel(x, y, c);
        }
        let _ = cx.shared.jdi.update_display();
    }

    #[task(binds = RTC0, local = [rtc_0])]
    fn rtc(cx: rtc::Context) {
        let _ = cx
            .local
            .rtc_0
            .is_event_triggered(hal::rtc::RtcInterrupt::Tick);
    }

    #[task(binds = SPIM3, shared = [jdi])]
    fn display_spi(cx: display_spi::Context) {
        cx.shared.jdi.spi_task_event();
    }
}
