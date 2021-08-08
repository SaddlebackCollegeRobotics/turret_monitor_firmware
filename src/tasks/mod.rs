//! This module contains RTIC tasks for doing various things.
//! Each task is in its own submodule, and is pub(crate) re-exported by this module for usage.
//!

/*
   private interface
*/

mod usart1_rx;
mod usart1_tx;

/// Task handling periodicly emitting current telemetry observations to the UART.
/// Note: this task requires a monotonic clock with at least 1s resolution.
mod write_telemetry;

/// Task handling reading the PWM input using advanced timer TIM8.
mod tim8;

/*
    public(crate) interface
*/
pub(crate) use tim8::tim8_cc;
pub(crate) use usart1_rx::{
    clear_idle_interrupt, enable_idle_interrupt, on_usart1_idle, on_usart1_rx_dma,
};
pub(crate) use usart1_tx::on_usart1_txe;
pub(crate) use write_telemetry::write_telemetry;
pub use write_telemetry::TxBufferState;
