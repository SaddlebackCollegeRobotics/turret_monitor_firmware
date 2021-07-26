#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused_imports)]

use panic_rtt_target as _panic_handler;

#[rtic::app(device = stm32f4xx_hal::stm32, peripherals = true)]
mod app {

    use stm32f4xx_hal::prelude::*;
    use stm32f4xx_hal::timer::Timer;
    use stm32f4xx_hal::pwm_input::PwmInput;
    use stm32f4xx_hal::stm32::TIM8;
    use stm32f4xx_hal::gpio::gpioc::PC6;
    use stm32f4xx_hal::gpio::Alternate;

    type PwmMonitor = PwmInput<TIM8, PC6<Alternate<3>>>;

    #[shared]
    struct Shared {
        monitor: PwmMonitor,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        // retrieve the RCC register, which is needed to obtain a handle to the clocks
        let rcc = ctx.device.RCC.constrain();
        // then retreive the clocks, so we can configure timers later on
        let clocks = rcc.cfgr.freeze();

        // obtain a reference to the GPIOC register block, so we can configure pins on the PC bus.
        let gpioc = ctx.device.GPIOC.split();

        // Configure one of TIM8's CH1 pins, so that its attached to the peripheral.
        // We need to do this since the pins are multiplexed across multiple peripherals
        let tim8_cc1 =gpioc.pc6.into_alternate();

        // Configure TIM8 into PWM input mode.
        // This requires a "best guess" of the input frequency in order to be accurate.
        // Note: as a side-effect TIM8's interrupt is enabled and fires whenever a capture-compare
        //      cycle is complete. See the reference manual's paragraphs on PWM Input.
        let monitor = Timer::new(ctx.device.TIM8, &clocks).pwm_input(240.hz(), tim8_cc1);

        // lastly return the shared and local resources, as per RTIC's spec.
            (Shared {
                monitor
            }, Local {}, init::Monotonics())
    }
}
