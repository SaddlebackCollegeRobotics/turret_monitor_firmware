use core::convert::TryInto;

use rtic::{mutex_prelude::*, time::duration::Seconds};
use rtt_target::rprintln;
use serde::{Deserialize, Serialize};
use serde_cbor::ser::{Serializer, SliceWrite};
use stm32f4xx_hal::{crc32::Crc32, prelude::*};

use crate::app::{Usart1Buf, Usart1TransferTx, Usart1Tx, BUF_SIZE, MESSAGE_SIZE, QeiMonitor};
use crate::datamodel::telemetry_packet::{TurretTelemetryPacket,TurretDirection };
use crate::tasks::usart1_rx::compute_crc;
use stm32f4xx_hal::hal::Direction;

pub enum TxBufferState {
    // Ready, use the contained buffer for next transfer
    Running(Usart1TransferTx),
    // In flight, but here is the next buffer to use.
    Idle(Usart1TransferTx),
}

pub(crate) fn write_telemetry(
    // unfortunately, as a implementation detail we cannot take an immutable reference
    // to this shared resource since another task mutates it. we have to take it mutably even
    // if we are only reading it.
    mut context: crate::app::write_telemetry::Context,
) {
    rprintln!("tick!");

    /*
        entering critical section
    */
    let monitor: &mut QeiMonitor = context.local.monitor;

    // retrieve the DMA state
    let dma_state: TxBufferState = context
        .shared
        .send
        .take()
        .expect("failed to aquire buffer state");

    // declare a buffer to fit the response in
    let mut payload_buffer: [u8; BUF_SIZE] = [0xFF; BUF_SIZE];
    // define the response
    let payload = TurretTelemetryPacket {
        turret_pos: monitor.count() as u32,
        turret_rot: match monitor.direction() {
            Direction::Downcounting => {TurretDirection::Backward}
            Direction::Upcounting => {TurretDirection::Forward}
        },
    };
    // set up serialization
    let mut serializer = Serializer::new(SliceWrite::new(&mut payload_buffer));
    // serialize payload
    if let Err(e) = payload.serialize(&mut serializer) {
        rprintln!("Failed to encode, error {:?}", e);
        return;
    }
    //
    let payload_size = serializer.into_inner().bytes_written();

    rprintln!("payload  before CRC := {:?}", payload_buffer);
    // sanity check.
    if payload_size > MESSAGE_SIZE - 4 {
        rprintln!("Encoded payload is too big! need at least 4 bytes to fit the CRC32!");
        return;
    }

    rprintln!("computing checksum for payload_size := {}", payload_size);
    /*
    entering critical section
     */
    let checksum: u32 = context
        .shared
        .crc
        .lock(|crc: &mut Crc32| compute_crc(&payload_buffer[..payload_size], crc));
    /*
    exiting critical section
     */
    rprintln!("sender CRC := {}", checksum);

    // append the CRC32 to the end.
    payload_buffer[payload_size..payload_size + 4].copy_from_slice(&checksum.to_be_bytes());
    rprintln!("buffer state before cobs := {:?}", payload_buffer);

    // if the DMA is idle, start a new transfer.
    if let TxBufferState::Idle(mut tx) = dma_state {
        rprintln!("DMA was idle, setting up next transfer...");
        // SAFETY: memory corruption can occur in double-buffer mode in the event of an overrun.
        //   - we are in single-buffer mode so this is safe.
        unsafe {
            // We re-use the existing DMA buffer, since the buffer has to live for 'static
            // in order to be safe. This was ensured during creation of the Transfer object,
            // so this is safe.
            tx.next_transfer_with(|buf, _| {
                // populate the DMA buffer with the new buffer's content
                postcard_cobs::encode(&payload_buffer[0..payload_size + 4], buf);
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
        *context.shared.send = Some(dma_state);
        rprintln!("[WARNING] write_Telemetry called but a previous USART1 DMA was still active!");
    };
    rprintln!("TX scheduled.");
}
