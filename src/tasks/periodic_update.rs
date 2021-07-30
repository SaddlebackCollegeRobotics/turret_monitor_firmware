use rtic::time::duration::Seconds;
use rtt_target::rprintln;

use crate::app::{Usart1DMATransferTx, Usart1Buf};
use rtic::mutex_prelude::*;
use stm32f4xx_hal::prelude::*;
use stm32f4xx_hal::pac::i2c1::cr1::POS_A::NEXT;

pub(crate) enum NextSerialBuffer {
    First,
    Second,
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

    let tx: &mut Usart1DMATransferTx = context.local.serial_tx_transfer;
    // figure out which buffer to use for this DMA transfer
    // (we have two, which must be alternated between transfers)
    let mut next_buffer: Usart1Buf = match context.local.serial_tx_next_buf {
        NextSerialBuffer::First => {
            context.local.serial_tx_next_buf = &mut NextSerialBuffer::Second;
            context.local.serial_tx_buf1
        }
        NextSerialBuffer::Second => {
            context.local.serial_tx_next_buf = &mut NextSerialBuffer::First;
            context.local.serial_tx_buf2
        }
    };
    // ensure the buffer is nulled out.
    next_buffer.fill(0x00);
    next_buffer[0..3].clone_from_slice(&turret_position.to_be_bytes());


    tx.next_transfer(next_buffer);
    // serial.write_fmt(format_args!("{}", turret_position)).expect("failed to write to UARt");

    reschedule_periodic();
}

fn reschedule_periodic() {
    // re-schedule this task (become periodic)
    crate::app::periodic_emit_status::spawn_after(Seconds(1u32))
        .expect("failed to re-spawn periodic task.");
}
