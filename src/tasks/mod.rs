//! This module contains RTIC tasks for doing various things.
//! Each task is in its own submodule, and is pub(crate) re-exported by this module for usage.
//!

/*
   private interface
*/

mod usart1_tx;
mod usart1_rx;

/// Task handling periodicly emitting current telemetry observations to the UART.
/// Note: this task requires a monotonic clock with at least 1s resolution.
mod periodic_update;

/// Task handling reading the PWM input using advanced timer TIM8.
mod tim8;

/*
    public(crate) interface
*/
pub(crate) use usart1_tx::on_usart1_txe;
pub(crate) use usart1_rx::on_usart1_rxne;
pub(crate) use periodic_update::periodic_emit_status;
pub use periodic_update::TxBufferState;
pub(crate) use tim8::tim8_cc;
