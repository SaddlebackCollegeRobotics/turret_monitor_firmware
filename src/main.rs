#![deny(unsafe_code)]
#![no_main]
#![no_std]
#![allow(unused_imports)]

use panic_rtt_target as _panic_handler;

/// submodule holding task handlers
mod tasks;

/*
 Declare the RTIC application itself.
 Firstly, we must provide it with the path to the device's PAC.
   - most HALs provide this as their `{hal}::stm32` module.
 We also want the device's peripherals, so we request those.
   - RTIC will provde these on the Context object of init.
 Lastly, we want to use some "software tasks", so we need to donate some unused interrupts to RTIC.
  - this is done via the `dispatchers` argument
*/
#[rtic::app(
    device = stm32f4xx_hal::stm32,
    peripherals = true,
    dispatchers=[SPI2, SPI3],
)]
mod app {
    use dwt_systick_monotonic::DwtSystick;
    use rtic::time::duration::Seconds;
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f4xx_hal::{
        gpio::{
            gpioc::{PC10, PC11, PC6},
            Alternate,
        },
        prelude::*,
        pwm_input::PwmInput,
        serial,
        stm32::{TIM8, UART4},
        timer::Timer,
    };
    const MONONTONIC_FREQ: u32 = 8_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<MONONTONIC_FREQ>;
    /* bring dependencies into scope */
    /// PWM input monitor type
    pub(crate) type PwmMonitor = PwmInput<TIM8, PC6<Alternate<3>>>;
    pub(crate) type Uart4 = serial::Serial<UART4, (PC10<Alternate<8>>, PC11<Alternate<8>>)>;

    /* resources shared across RTIC tasks */
    #[shared]
    struct Shared {
        /// the last observed position of the turret
        last_observed_turret_position: f32,
    }

    /* resources local to specific RTIC tasks */
    #[local]
    struct Local {
        monitor: PwmMonitor,
        serial: Uart4,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        /*
            This patch enables the debugger to behave correctly during a WFI
            See Errata: https://www.st.com/content/ccc/resource/technical/document/errata_sheet/c3/6b/f8/32/fc/01/48/6e/DM00155929.pdf/files/DM00155929.pdf/jcr:content/translations/en.DM00155929.pdf#%5B%7B%22num%22%3A37%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C67%2C724%2Cnull%5D
            See Also Github: https://github.com/probe-rs/probe-rs/issues/350#issuecomment-740550519
        */
        // enable the dma1 master
        ctx.device.RCC.ahb1enr.modify(|_, w| w.dma1en().enabled());
        // enable the debugger.
        ctx.device.DBGMCU.cr.modify(|_, w| {
            w.dbg_sleep().set_bit();
            w.dbg_standby().set_bit();
            w.dbg_stop().set_bit()
        });

        // Enable RTT logging
        rtt_init_print!();
        rprintln!("hello, world!");
        // retrieve the RCC register, which is needed to obtain a handle to the clocks
        let rcc = ctx.device.RCC.constrain();
        // then retreive the clocks, so we can configure timers later on
        let clocks = rcc.cfgr.freeze();

        /* start RTIC monotonics */
        // configure RTIC's monotonic using the system tick.
        // Note: this has a maximum duration of ~20 seconds, so it can't be used for super long
        // delays.
        let mut dcb = ctx.core.DCB;
        let dwt = ctx.core.DWT;
        let systick = ctx.core.SYST;
        let mono = DwtSystick::new(&mut dcb, dwt, systick, MONONTONIC_FREQ);
        /* end RTIC monotonics */

        // obtain a reference to the GPIOC register block, so we can configure pins on the PC bus.
        let gpioc = ctx.device.GPIOC.split();

        // Configure one of TIM8's CH1 pins, so that its attached to the peripheral.
        // We need to do this since the pins are multiplexed across multiple peripherals
        let tim8_cc1 = gpioc.pc6.into_alternate();

        // Configure TIM8 into PWM input mode.
        // This requires a "best guess" of the input frequency in order to be accurate.
        // Note: as a side-effect TIM8's interrupt is enabled and fires whenever a capture-compare
        //      cycle is complete. See the reference manual's paragraphs on PWM Input.
        let monitor = Timer::new(ctx.device.TIM8, &clocks).pwm_input(240.hz(), tim8_cc1);

        // configure UART4.
        // This is the primary interface to this driver.
        let uart4_tx = gpioc.pc10.into_alternate();
        let uart4_rx = gpioc.pc11.into_alternate();
        let uart4_config = serial::config::Config {
            baudrate: 9600.bps(),
            wordlength: serial::config::WordLength::DataBits8,
            parity: serial::config::Parity::ParityNone,
            stopbits: serial::config::StopBits::STOP1,
            dma: serial::config::DmaConfig::None,
        };
        let uart4 =
            serial::Serial::new(ctx.device.UART4, (uart4_tx, uart4_rx), uart4_config, clocks)
                .expect("failed to configure UART4.");

        // kick off the periodic task.
        periodic_emit_status::spawn_after(Seconds(1u32))
            .expect("failed to kick off periodic task.");
        // lastly return the shared and local resources, as per RTIC's spec.
        (
            Shared {
                last_observed_turret_position: 0.0,
            },
            Local {
                monitor,
                serial: uart4,
            },
            init::Monotonics(mono),
        )
    }

    /* bring externed tasks into scope */
    use crate::tasks::{periodic_emit_status, tim8_cc};

    // RTIC docs specify we can modularize the code by using these `extern` blocks.
    // This allows us to specify the tasks in other modules and still work within
    // RTIC's infrastructure.
    extern "Rust" {
        // PWM Monitor interrupt handler
        #[task(binds=TIM8_CC, local=[monitor], shared=[last_observed_turret_position])]
        fn tim8_cc(context: tim8_cc::Context);

        // periodic UART telemetry output task
        #[task(shared=[last_observed_turret_position])]
        fn periodic_emit_status(context: periodic_emit_status::Context);
    }
}
