#![no_main]
#![no_std]

use microbit_two::hal;
use microbit_two::hal::pac;
use rtic::app;

#[app(device = pac, peripherals = true)]
mod app {
    use super::{hal, pac};

    use embedded_graphics::{
        geometry::{Point, Size},
        primitives::{Circle, Primitive, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Styled},
        Drawable,
    };
    use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
    use hal::{
        clocks,
        gpio::{self, Output, PushPull},
        timer::Instance,
    };
    use microbit_two::{
        lpm013m126a::{self, Palette8, DISPLAY_HEIGHT, DISPLAY_WIDTH},
        spim,
    };
    use pac::{RTC0, TIMER0, TIMER1, TIMER2};

    pub struct DrawContext {
        pub circle: Circle,
        pub circle_style: PrimitiveStyle<Palette8>,
        pub background: Styled<Rectangle, PrimitiveStyle<Palette8>>,
        pub background_style: PrimitiveStyle<Palette8>,
    }

    #[local]
    struct Local {
        rtc_0: hal::rtc::Rtc<RTC0>,
        timer_0: TIMER0,
        timer_1: TIMER1,
        timer_2: TIMER2,
        led_matrix: microbit_two::LedMatrix,
        jdi_com: hal::gpio::Pin<hal::gpio::Output<hal::gpio::PushPull>>,
        draw_context: DrawContext,
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

        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                jdi.set_pixel(x, y, lpm013m126a::Palette8::Green);
            }
        }

        let circle = Circle::new(Point::new(22, 22), 20);

        let background_style = PrimitiveStyleBuilder::new()
            .stroke_width(5)
            .stroke_color(Palette8::Black)
            .fill_color(Palette8::Black)
            .build();
        let background_rectangle = Rectangle::new(
            Point::new(0, 0),
            Size::new(u32::from(DISPLAY_WIDTH), u32::from(DISPLAY_HEIGHT)),
        );
        let background = background_rectangle.into_styled(background_style);
        let _ = background.draw(&mut jdi);

        let circle_style = PrimitiveStyleBuilder::new()
            .stroke_width(3)
            .stroke_color(Palette8::Cyan)
            .build();

        defmt::info!("Initialized");

        let shared = Shared { jdi };
        let local = Local {
            timer_0: cx.device.TIMER0,
            timer_1: cx.device.TIMER1,
            timer_2: cx.device.TIMER2,
            rtc_0,
            led_matrix,
            jdi_com,
            draw_context: DrawContext {
                circle,
                circle_style,
                background,
                background_style,
            },
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

    #[task(binds = TIMER2, local = [timer_2, draw_context], shared = [jdi])]
    fn timer2(cx: timer2::Context) {
        let ctx = cx.local.draw_context;

        let _ = ctx
            .circle
            .into_styled(ctx.background_style)
            .draw(cx.shared.jdi);

        ctx.circle.top_left += Point::new(1, 1);
        if ctx.circle.top_left.x >= lpm013m126a::DISPLAY_WIDTH as i32 {
            ctx.circle.top_left.x = 0;
        }
        if ctx.circle.top_left.y >= lpm013m126a::DISPLAY_HEIGHT as i32 {
            ctx.circle.top_left.y = 0;
        }

        let _ = ctx.circle.into_styled(ctx.circle_style).draw(cx.shared.jdi);

        let _ = cx.shared.jdi.update_display();

        cx.local.timer_2.timer_reset_event();
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
