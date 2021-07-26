#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused_imports)]

use panic_rtt_target as _panic_handler;

#[rtic::app(device = stm32f4xx_hal::stm32, peripherals = true)]
mod app {

    use stm32f4xx_hal::prelude::*;
    use stm32f4xx_hal::timer::Timer;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.freeze();

        let gpioc = ctx.device.GPIOC.split();

        let tim8_cc1 =gpioc.pc6.into_alternate();

        let monitor = Timer::new(ctx.device.TIM8, &clocks).pwm_input(240.hz(), tim8_cc1);
            (Shared {}, Local {}, init::Monotonics())
    }
}
