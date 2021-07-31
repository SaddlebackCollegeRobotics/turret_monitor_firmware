use rtic::time::duration::Seconds;
use rtt_target::rprintln;

use crate::app::{Usart1, Usart1Buf, Usart1DMATransferTx, BUF_SIZE};
use rtic::mutex_prelude::*;
use serde::{Deserialize, Serialize};
use stm32f4xx_hal::prelude::*;

const PERODIC_DELAY: Seconds = Seconds(1u32);

pub enum TxBufferState {
    // Ready, use the contained buffer for next transfer
    Running(Usart1DMATransferTx),
    // In flight, but here is the next buffer to use.
    Idle(Usart1DMATransferTx),
}

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    turret_pos: f32,
}

pub(crate) fn periodic_emit_status(
    // unfortunately, as a implementation detail we cannot take an immutable reference
    // to this shared resource since another task mutates it. we have to take it mutably even
    // if we are only reading it.
    mut context: crate::app::periodic_emit_status::Context,
) {
    rprintln!("tick!");

    /*
        entering critical section
    */
    let turret_position: f32 = context
        .shared
        .last_observed_turret_position
        .lock(|guard| *guard);

    /*
        leaving critical section
    */
    // retrieve the DMA state
    let dma_state: TxBufferState = context
        .shared
        .send
        .take()
        .expect("failed to aquire buffer state");

    // declare a buffer to fit the response in
    let mut payload_buffer: [u8; BUF_SIZE] = [0xFF; BUF_SIZE];
    // define the response
    let payload = Payload {
        turret_pos: turret_position,
    };
    // attempt to serialize the response
    if let Err(e) = serde_json_core::to_slice(&payload, &mut payload_buffer) {
        rprintln!("Failed to encode, error {:?}", e);
        // bail out without panicking.
        return reschedule_periodic();
    }
    rprintln!("payload := {:?}", payload);

    // if the DMA is idle, start a new transfer.
    if let TxBufferState::Idle(mut tx) = dma_state {
        rprintln!("DMA was idle, setting up next transfer...");
        // SAFETY: memory corruption can occur in double-buffer mode in the event of an overrun.
        //   - we are in single-buffer mode so this is safe.
        unsafe {
            // We re-use the existing DMA buffer, since the buffer has to live for 'static
            // in order to be safe. This was ensured during creation of the Transfer object.
            tx.next_transfer_with(|buf, _| {
                // populate the DMA buffer with the new bufer's content
                buf.copy_from_slice(&payload_buffer);
                // log the TX buffer
                rprintln!("buf :: {:?}", buf);
                // calculate the buffer's length, if only to satisfy the closure's contract.
                let buf_len = buf.len();
                (buf, buf_len) // Don't know what the second argument is, but it seems to be ignored.
            })
            .expect("Something went horribly wrong setting up the transfer.");
        }
        // update the DMA state into the running phase
        *context.shared.send = Some(TxBufferState::Running(tx));
    } else {
        rprintln!("[WARNING] periodic ticked but a previous DMA was still active!");
    };

    // serial.write_fmt(format_args!("{}", turret_position)).expect("failed to write to UARt");

    reschedule_periodic();
}

fn reschedule_periodic() {
    // re-schedule this task (become periodic)
    crate::app::periodic_emit_status::spawn_after(PERODIC_DELAY)
        .expect("failed to re-spawn periodic task.");
}
