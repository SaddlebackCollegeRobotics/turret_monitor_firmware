//! This module contains RTIC tasks for doing various things.
//! Each task is in its own submodule, and is pub(crate) re-exported by this module for usage.
//!

/*
   private interface
*/

/// Task handling periodicly emitting current telemetry observations to the UART.
/// Note: this task requires a monotonic clock with at least 1s resolution.
mod periodic_update;
mod dma2_stream7;

/// Task handling reading the PWM input using advanced timer TIM8.
mod tim8;

/*
    public(crate) interface
*/
pub(crate) use periodic_update::periodic_emit_status;
pub use periodic_update::TxBufferState;
pub(crate) use tim8::tim8_cc;
pub(crate) use dma2_stream7::on_dma2_stream7;
