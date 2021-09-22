#![no_main]
#![no_std]
#![allow(unused_imports)]

use panic_rtt_target as _panic_handler;

mod datamodel;
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
dispatchers = [SPI2, SPI3],
)]
mod app {
    /* bring dependencies into scope */

    use crate::tasks::TxBufferState;
    use cortex_m::singleton;
    use dwt_systick_monotonic::DwtSystick;
    use rtic::time::duration::Seconds;
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f4xx_hal::{
        crc32::Crc32,
        dma::{
            config::DmaConfig, Channel7, MemoryToPeripheral, PeripheralToMemory, Stream2, Stream7,
            StreamsTuple, Transfer,
        },
        gpio::{
            gpioc::{PC10, PC11, PC6, PC7},
            Alternate,
        },
        prelude::*,
        pwm::{PwmChannels, C1},
        pwm_input::PwmInput,
        rcc::Rcc,
        serial,
        stm32::{DMA2, TIM4, TIM8, USART1},
        timer::Timer,
    };

    /*
    Monotonic config
     */
    const MONONTONIC_FREQ: u32 = 8_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<MONONTONIC_FREQ>;

    /*
    Peripheral type definitions
     */
    /// PWM input monitor type
    pub(crate) type QeiMonitor = Qei<TIM8, (PC6<Alternate<3>>, PC7<Alternate<3>>)>;
    /// Serial connection type
    pub(crate) type Usart1Tx = serial::Tx<USART1>;
    pub(crate) type Usart1Rx = serial::Rx<USART1>;
    /*
    USART DMA definitions
     */
    /// Size of USART1's DMA buffer
    pub(crate) const BUF_SIZE: usize = 32;
    /// Maximum message size for messages on USART1.
    pub(crate) const MESSAGE_SIZE: usize = BUF_SIZE - 1;

    /// USART1's DMA buffer type
    pub(crate) type Usart1Buf = &'static mut [u8; BUF_SIZE];

    /// Serial TX DMA type
    pub(crate) type Usart1TransferTx =
    Transfer<Stream7<DMA2>, Usart1Tx, MemoryToPeripheral, Usart1Buf, 4>;

    /// Serial RX DMA type
    pub(crate) type Usart1TransferRx =
    Transfer<Stream2<DMA2>, Usart1Rx, PeripheralToMemory, Usart1Buf, 4>;

    /* resources shared across RTIC tasks */
    #[shared]
    struct Shared {
        /// the last observed position of the turret
        last_observed_turret_position: f32,

        #[lock_free]
        send: Option<TxBufferState>,
        crc: Crc32,
        recv: Usart1TransferRx,
    }

    /* resources local to specific RTIC tasks */
    #[local]
    struct Local {
        monitor: QeiMonitor,
    }

    /*
    The init task, called once at startup.
     The locals have 'static storage, which make them suitable for usage with DMA.
     */

    #[init(
    local = [
    tx_buf: [u8; BUF_SIZE] = [0; BUF_SIZE],
    rx_buf: [u8; BUF_SIZE] = [0; BUF_SIZE],
    ]
    )]
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
        let rcc: Rcc = ctx.device.RCC.constrain();
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

        // Configure one of TIM8's CH1 pins, so that its attached to the peripheral.
        // We need to do this since the pins are multiplexed across multiple peripherals

        // Configure TIM8 into PWM input mode.
        // This requires a "best guess" of the input frequency in order to be accurate.
        // Note: as a side-effect TIM8's interrupt is enabled and fires whenever a capture-compare
        //      cycle is complete. See the reference manual's paragraphs on PWM Input.

        let monitor = Qei::new(ctx.device.TIM8, (gpioc.pc6.into_alternate(), gpioc.pc7.into_alternate()));

        let mut pwm_mock: PwmChannels<TIM4, C1> =
            Timer::new(ctx.device.TIM4, &clocks).pwm(gpiob.pb6.into_alternate(), 200.hz());
        pwm_mock.set_duty(pwm_mock.get_max_duty() / 2);
        // pwm_mock.enable();

        /*
        begin USART1 config
         */

        // This is the primary interface to this driver.
        let usart1_tx_pin = gpioa.pa9.into_alternate();
        let usart1_rx_pin = gpioa.pa10.into_alternate();
        let usart1_config = serial::config::Config {
            baudrate: 115200.bps(),
            wordlength: serial::config::WordLength::DataBits8,
            parity: serial::config::Parity::ParityNone,
            stopbits: serial::config::StopBits::STOP1,
            dma: serial::config::DmaConfig::TxRx,
        };
        let (usart1_tx, usart1_rx) = serial::Serial::new(
            ctx.device.USART1,
            (usart1_tx_pin, usart1_rx_pin),
            usart1_config,
            clocks,
        )
            .expect("failed to configure UART4.")
            .split();

        // set up the DMA transfers.
        let dma2_streams: StreamsTuple<DMA2> = StreamsTuple::new(ctx.device.DMA2);

        let usart1_dma_tx_config = DmaConfig::default()
            .transfer_complete_interrupt(true)
            .memory_increment(true);

        let usart1_dma_rx_config = DmaConfig::default()
            // these should never fire in a well-formed packet.
            .transfer_complete_interrupt(false)
            .half_transfer_interrupt(false)
            // enable the interrupt when something goes horribly wrong.
            .fifo_error_interrupt(true)
            .transfer_error_interrupt(true)
            .direct_mode_error_interrupt(true)
            .memory_increment(true);

        let usart1_dma_transfer_tx: Usart1TransferTx = Transfer::init_memory_to_peripheral(
            dma2_streams.7,
            usart1_tx,
            ctx.local.tx_buf,
            None,
            usart1_dma_tx_config,
        );

        let mut usart1_dma_transfer_rx: Usart1TransferRx = Transfer::init_peripheral_to_memory(
            dma2_streams.2,
            usart1_rx,
            ctx.local.rx_buf,
            None,
            usart1_dma_rx_config,
        );
        unsafe {
            crate::tasks::enable_idle_interrupt();
        }

        usart1_dma_transfer_rx.start(|_rx| {
            rprintln!("started RX DMA.");
        });
        /*
        End USART1 configuration.
        */

        // set up the CRC32 (ethernet) peripheral
        let crc = Crc32::new(ctx.device.CRC);

        // kick off the periodic task.
        write_telemetry::spawn_after(Seconds(1u32)).expect("failed to kick off periodic task.");
        // lastly return the shared and local resources, as per RTIC's spec.
        (
            Shared {
                last_observed_turret_position: 0.0,
                send: Some(TxBufferState::Idle(usart1_dma_transfer_tx)),
                crc,
                recv: usart1_dma_transfer_rx,
            },
            Local { monitor },
            init::Monotonics(mono),
        )
    }

    /* bring externed tasks into scope */
    use crate::tasks::{on_usart1_idle, on_usart1_rx_dma, on_usart1_txe, tim8_cc, write_telemetry};
    use stm32f4xx_hal::qei::Qei;

    // RTIC docs specify we can modularize the code by using these `extern` blocks.
    // This allows us to specify the tasks in other modules and still work within
    // RTIC's infrastructure.
    extern "Rust" {
        // PWM Monitor interrupt handler
        #[task(binds = TIM8_CC, local = [monitor], shared = [last_observed_turret_position])]
        fn tim8_cc(context: tim8_cc::Context);

        // periodic UART telemetry output task
        #[task(
        shared = [last_observed_turret_position, send, crc]
        )]
        fn write_telemetry(context: write_telemetry::Context);

        // when USART1 is done sending data
        #[task(
        binds = DMA2_STREAM7,
        shared = [send]
        )]
        fn on_usart1_txe(context: on_usart1_txe::Context);

        #[task(
        binds = DMA2_STREAM2,
        shared = [crc, recv],
        )]
        // when USART1 is done receiving data
        fn on_usart1_rx_dma(context: on_usart1_rx_dma::Context);
        #[task(
        binds = USART1,
        shared = [recv, crc]
        )]
        fn on_usart1_idle(context: on_usart1_idle::Context);
    }
}
