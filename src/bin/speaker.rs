#![no_main]
#![no_std]

use microbit_two;

use rtic::app;

use crate::hal::pac;
use microbit_two::hal;

use hal::{clocks, gpio, pwm, time::U32Ext, timer::Instance, prelude::OutputPin};
use pac::{RTC0, TIMER0};

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        rtc_0: hal::rtc::Rtc<RTC0>,
        timer_0: TIMER0,
        led_matrix: microbit_two::LedMatrix,
        speaker: pwm::Pwm<pac::PWM0>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {

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

        let mut speaker_pin = port0
            .p0_00
            .into_push_pull_output(gpio::Level::Low);

        let _ = speaker_pin.set_low();
        // Set high drive for speaker pin H0H1 (3) in PIN_CNF.

        let ring_0 = port0
            .p0_02
            .into_push_pull_output(gpio::Level::Low);
        let speaker = pwm::Pwm::new(cx.device.PWM0);
        speaker
            .set_output_pin(pwm::Channel::C0, &speaker_pin.degrade())
            .set_output_pin(pwm::Channel::C1, &ring_0.degrade())
            .set_prescaler(pwm::Prescaler::Div2)
            .set_period(1000u32.hz())
            .set_counter_mode(pwm::CounterMode::UpAndDown)
            .enable();

        speaker
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            .set_seq_refresh(pwm::Seq::Seq1, 0)
            .set_seq_end_delay(pwm::Seq::Seq0, 0)
            .set_seq_end_delay(pwm::Seq::Seq1, 0);

        let max_duty = speaker.max_duty();

        speaker.set_duty_on_common(max_duty / 2);

        led_matrix.display(microbit_two::images::SCALES);

        init::LateResources {
            timer_0: cx.device.TIMER0,
            rtc_0,
            led_matrix,
            speaker,
        }
    }

    #[task(binds = TIMER0, resources = [timer_0, led_matrix])]
    fn timer(cx: timer::Context) {
        cx.resources.timer_0.timer_reset_event();
        cx.resources.led_matrix.update();
    }

    #[task(binds = RTC0, resources = [rtc_0, led_matrix])]
    fn rtc(cx: rtc::Context) {
        let _ = cx
            .resources
            .rtc_0
            .is_event_triggered(hal::rtc::RtcInterrupt::Tick);
    }
};
