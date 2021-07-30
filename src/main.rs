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
    use cortex_m::singleton;
    use dwt_systick_monotonic::DwtSystick;
    use rtic::time::duration::Seconds;
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f4xx_hal::{
        dma::{config::DmaConfig, Channel7, MemoryToPeripheral, Stream7, StreamsTuple, Transfer},
        gpio::{
            gpioc::{PC10, PC11, PC6},
            Alternate,
        },
        prelude::*,
        pwm_input::PwmInput,
        serial,
        stm32::{DMA2, TIM8, USART1},
        timer::Timer,
    };

    const MONONTONIC_FREQ: u32 = 8_000_000;
    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<MONONTONIC_FREQ>;
    /* bring dependencies into scope */
    /// PWM input monitor type
    pub(crate) type PwmMonitor = PwmInput<TIM8, PC6<Alternate<3>>>;
    /// Serial connection type
    pub(crate) type Usart1 = serial::Tx<USART1>;
    pub(crate) type Usart1Buf = &'static mut [u8;32];
    /// Serial TX DMA type
    pub(crate) type Usart1DMATransferTx =
        Transfer<Stream7<DMA2>, Usart1, MemoryToPeripheral, Usart1Buf, 4>;

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
        serial_tx_transfer: Usart1DMATransferTx,
        serial_tx_buf1: Usart1Buf,
        serial_tx_buf2: Usart1Buf,
        serial_tx_next_buf: crate::tasks::NextSerialBuffer,
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

        // obtain a reference to the GPIO* register blocks, so we can configure pins on the P* buses.
        let gpioc = ctx.device.GPIOC.split();
        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        // obtain

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
        let usart1_tx = gpioa.pa9.into_alternate();
        let usart1_config = serial::config::Config {
            baudrate: 9600.bps(),
            wordlength: serial::config::WordLength::DataBits8,
            parity: serial::config::Parity::ParityNone,
            stopbits: serial::config::StopBits::STOP1,
            dma: serial::config::DmaConfig::Tx,
        };
        let usart1: Usart1 =
            serial::Serial::tx(ctx.device.USART1, usart1_tx, usart1_config, clocks)
                .expect("failed to configure UART4.");

        // set up the DMA transfer.
        let dma2_streams: StreamsTuple<DMA2> = StreamsTuple::new(ctx.device.DMA2);
        let dma1_stream4_config = DmaConfig::default()
            .transfer_error_interrupt(true)
            .double_buffer(true);
        let usart1_tx_buf1: Usart1Buf = singleton!(: [u8; 32] = [0; 32]).unwrap();
        let usart1_tx_buf2: Usart1Buf = singleton!(: [u8; 32] = [0; 32]).unwrap();
        let usart1_dma_transfer_tx = Transfer::init_memory_to_peripheral(
            dma2_streams.7,
            usart1,
            usart1_tx_buf1,
            Some(usart1_tx_buf2),
            dma1_stream4_config,
        );

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
                serial_tx_transfer: usart1_dma_transfer_tx,

                serial_tx_buf1: usart1_tx_buf1,
                serial_tx_buf2: usart1_tx_buf2,
                serial_tx_next_buf: crate::tasks::NextSerialBuffer::First
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
        #[task(
            shared=[last_observed_turret_position],
            local=[
                serial_tx_transfer,
                serial_tx_next_buf,
                serial_tx_buf1,
                serial_tx_buf2,
            ])]
        fn periodic_emit_status(context: periodic_emit_status::Context);
    }
}
