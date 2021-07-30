use rtic::time::duration::Seconds;
use rtt_target::rprintln;

use crate::app::{Usart1DMATransferTx, Usart1Buf, Usart1, BUF_SIZE};
use rtic::mutex_prelude::*;
use stm32f4xx_hal::prelude::*;
use embedded_dma::WriteTarget;

pub enum TxBufferState {
    // Ready, use the contained buffer for next transfer
    Running(Usart1DMATransferTx),
    // In flight, but here is the next buffer to use.
    Idle(Usart1DMATransferTx),
}

pub(crate) fn periodic_emit_status(
    // unfortunately, as a implementation detail we cannot take an immutable reference
    // to this shared resource since another task mutates it. we have to take it mutably even
    // if we are only reading it.
    mut context: crate::app::periodic_emit_status::Context,
) {
    rprintln!("tick!");

    let mut turret_position: f32 = 0.0;
    /*
        entering critical section
    */
    context.shared.last_observed_turret_position.lock(|guard| {
        turret_position = *guard;
    });
    /*
        leaving critical section
    */

    let dma_state: TxBufferState = context.shared.send.take().expect("failed to aquire buffer state");
    if let TxBufferState::Idle(mut tx) = dma_state {
        rprintln!("DMA was idle, setting up next transfer...");

        unsafe {
            tx.next_transfer_with(|buf, _| {
                // buf[0..4].clone_from_slice(&turret_position.to_be_bytes());
                // buf[5] = ',' as u8;
                buf.fill(0xAF);
                let buf_len= buf.len();
                    (buf, buf_len)
            }) .expect("Something went horribly wrong setting up the transfer.");
        }
        *context.shared.send = Some(TxBufferState::Running(tx));
    };

    // serial.write_fmt(format_args!("{}", turret_position)).expect("failed to write to UARt");

    reschedule_periodic();
}

fn reschedule_periodic() {
    // re-schedule this task (become periodic)
    crate::app::periodic_emit_status::spawn_after(Seconds(1u32))
        .expect("failed to re-spawn periodic task.");
}
