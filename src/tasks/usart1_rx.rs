use rtic::mutex_prelude::*;
use rtt_target::rprintln;
use stm32f4xx_hal::dma::{traits::*, Stream2};
use stm32f4xx_hal::stm32::{DMA2, USART1};

use crate::app::{
    on_usart1_idle, on_usart1_rx_dma, Usart1Buf, Usart1TransferRx, {BUF_SIZE, MESSAGE_SIZE},
};
use crate::datamodel::{request::Request, rx_errors::RxError};
use crate::tasks::TxBufferState;
use core::convert::TryInto;
use core::ops::Index;
use stm32f4xx_hal::crc32::Crc32;

/// Handles the DMA transfer complete Interrupt
pub(crate) fn on_usart1_rx_dma(_ctx: on_usart1_rx_dma::Context) {
    rprintln!("DMA error occured!");
}

/// handles USART1 IDLE interrupt
/// This fires when the host starts sending data but then stops
/// before the transfer completes (e.g. sends 12 bytes when BUF_SIZE > 12).
pub(crate) fn on_usart1_idle(ctx: on_usart1_idle::Context) {
    rprintln!("RX line fell idle, packet recv'ed.");
    // acquire lock to shared resources, then call the actual handler.
    (ctx.shared.recv, ctx.shared.crc).lock(|transfer: &mut Usart1TransferRx, crc: &mut Crc32| {
        handle_rx(transfer, crc);
    });
}

/// Actually handles the received packet, regardless of its source.
fn handle_rx(transfer: &mut Usart1TransferRx, crc: &mut Crc32) {
    let remaining_transfers = Stream2::<DMA2>::get_number_of_transfers() as usize;
    let bytes_transfered = BUF_SIZE - remaining_transfers;

    rprintln!(
        "RX dma remaining transfers := {},bytes transfered:={}",
        remaining_transfers,
        bytes_transfered
    );

    let mut packet = [0u8; BUF_SIZE];
    // NOTE(unsafe): only unsafe in the event of a overrun in double-buffer mode.
    if let Err(e) = unsafe {
        // set up the next transfer, and copy the previous transfer to a different buffer for
        // further processing
        transfer.next_transfer_with(|buf, _current_buffer| {
            rprintln!("Buf current reads := {:?}", buf);
            let len = buf.len();
            /*
            Fetch any DMA errors that might have occured.
            For some reason these arn't exposed on the transfer interface so we need to fetch them
            directly from the source.

            NOTE(safety): atomic reads with no side effects.
             */
            let direct_mode_error = Stream2::<DMA2>::get_direct_mode_error_flag();
            let transfer_error = Stream2::<DMA2>::get_transfer_error_flag();
            let fifo_error = Stream2::<DMA2>::get_fifo_error_flag();
            if direct_mode_error || transfer_error || fifo_error {
                rprintln!(
                    "DMA transfer error occured! direct mode:={},transfer:={},fifo:={}",
                    direct_mode_error,
                    transfer_error,
                    fifo_error
                );
            }
            // Copy DMA buffer to a different location so DMA can be restarted while
            // we process the packet.
            packet.copy_from_slice(buf);
            (buf, len)
        })
    } {
        rprintln!("something went horribly wrong in DMA reconfig! {:?}", e);
        transfer.clear_interrupts();
        unsafe { clear_idle_interrupt() };
        return;
    }
    // Now that the RXed buffer is copied into our local buffer, and the DMA is reconfigured
    //
    let result = if bytes_transfered > MESSAGE_SIZE {
        rprintln!("Someone sent a bigger message frame than allowed.");
        Err(RxError::BufferOverflow)
    } else {
        process_mabie_packet(&packet, crc)
    };
    if let Err(e) = result {
        rprintln!(
            "[ERROR] Something went horribly wrong processing packet {:?}!",
            e
        );
    }

    transfer.clear_interrupts();
    unsafe { clear_idle_interrupt() };
}

/*
USART hackery
 */

#[inline]
/// Does the special read sequence to clear the USART1 Idle interrupt as described in the RM.
/// SAFETY:
/// Atomic reads, resets the IDLE USART1 interrupt.
pub(crate) unsafe fn clear_idle_interrupt() {
    let _ = (*USART1::ptr()).sr.read().idle();
    let _ = (*USART1::ptr()).dr.read().bits();
}

#[inline]
/// Enables USART1's IDLE interrupt
/// SAFETY:
/// read/modify/write cycle
pub(crate) unsafe fn enable_idle_interrupt() {
    (*USART1::ptr()).cr1.modify(|_, w| w.idleie().set_bit());
}

fn process_mabie_packet(input_buffer: &[u8], crc: &mut Crc32) -> Result<(), RxError> {
    let mut buffer: [u8; BUF_SIZE] = [0; BUF_SIZE];

    let mut decoder = postcard_cobs::CobsDecoder::new(&mut buffer);

    // decode the COBS frame into the buffer
    if let Ok(n) = match decoder.push(input_buffer) {
        Ok(None) => {
            rprintln!("[ERROR] Decoder demanded more bytes than we can feed it.");
            Err(RxError::CobsDecoderNeededMoreBytes)
        }
        Ok(Some((message_length, _))) => {
            rprintln!("Decode successful, decoded {} bytes.", message_length);
            rprintln!("un-COBS'ed := {:?}", buffer);
            Ok(message_length)
        }
        Err(j) => {
            rprintln!("[ERROR] Decoder errored after {} bytes.", j);
            Err(RxError::CobsDecoderError(j))
        }
    } {
        // NOTE: this somehow solves an off-by-one error.
        let n = n - 1;
        // If decoding succeeded, then fetch the sender CRC.
        let crc_bytes = &buffer[n - 4..n];
        rprintln!("crc buffer := {:?}", crc_bytes);
        let sender_crc = u32::from_be_bytes(
            crc_bytes
                .try_into()
                .expect("failed to interpret sender CRC as a u32!"),
        );
        // Then compute the device CRC.
        let data = &mut buffer[..n - 4];
        rprintln!("computing sender CRC with data length {}", data.len());
        let device_crc = compute_crc(data, crc);

        // Ensure the two match..
        if sender_crc != device_crc {
            rprintln!(
                "[ERROR] Sender CRC {} != Device CRC {}",
                sender_crc,
                device_crc
            );
            Err(RxError::InvalidSenderCrc)
        } else {
            rprintln!("RX checksum passed.");
            // Deserialize internal CBOR packet.
            // Note: the data buffer needs to be mutable as an implementation detail of CBOR.
            let request_result: serde_cbor::Result<Request> = serde_cbor::de::from_mut_slice(data);

            // Check that the deserialization was successful.
            if let Ok(request) = request_result {
                rprintln!("successfully deserialized request {:?}", request);
                // Spawn the telemetry worker
                // Note: we remap the error here to our internal enum for consistancy.
                crate::app::write_telemetry::spawn().map_err(|e| {
                    rprintln!("[error] failed to spawn telemetry writer with err {:?}", e);
                    RxError::FailedTelemetrySpawn
                })?;
                Ok(())
            } else {
                rprintln!("[error] failed to deserialize well-formed packet!");
                Err(RxError::FailedDeserialize)
            }
        }
    } else {
        Err(RxError::CobsDecoderPushFailed)
    }
}

/// computes the CRC-32(ethernet) of the provided data buffer.
/// Note: this uses the CRC32 peripheral, which only operates on u32 words.
///     For the sake of simplicity, the input buffer is truncated to the nearest word boundry,
///     and the resulting smaller buffer is then fed to the peripheral.
pub(crate) fn compute_crc(buffer: &[u8], crc: &mut Crc32) -> u32 {
    // Reset the peripheral.
    crc.init();
    let payload_size = buffer.len();
    let remainder = payload_size % 4;
    let total_words = payload_size / 4;
    if remainder != 0 {
        rprintln!("input data (length {}) was not word-aligned, truncating to {} bytes for calculation...", buffer.len(), total_words*4)
    }
    // truncate to the word boundry
    let buffer = &buffer[0..total_words * 4];
    let chunks = buffer.chunks_exact(4);

    rprintln!(
        "checksumming {} bytes of a {} byte payload across {} words.",
        buffer.len(),
        payload_size,
        chunks.len()
    );
    rprintln!("buffer := {:?}", buffer);
    let mut result: u32 = 0;
    chunks.for_each(|chunk| {
        let word = u32::from_be_bytes(chunk.try_into().expect("unexpected misalligned word."));
        rprintln!("feeding word {:x}", word);
        result = crc.update(&[word])
    });

    result
}
