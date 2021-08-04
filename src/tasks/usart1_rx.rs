use rtic::{mutex_prelude::*};
use rtt_target::rprintln;
use stm32f4xx_hal::dma::{Stream4, StreamX, traits::StreamISR};
use stm32f4xx_hal::stm32::{DMA2, USART1};

use crate::app::{on_usart1_idle, on_usart1_rxne, Usart1TransferRx};
use crate::tasks::TxBufferState;

pub(crate) fn on_usart1_rxne(mut ctx: on_usart1_rxne::Context) {
    rprintln!("Full packet Recevied!");
    ctx.shared.recv.lock(|transfer: &mut Usart1TransferRx| {
        handle_rx(transfer);
    });
}

pub(crate) fn on_usart1_idle(mut ctx: on_usart1_idle::Context) {
    rprintln!("RX line fell idle, packet recv'ed.");
    ctx.shared.recv.lock(|transfer: &mut Usart1TransferRx| {
        handle_rx(transfer);
    });
}

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


fn handle_rx(transfer: &mut Usart1TransferRx) {
    // NOTE(unsafe): only unsafe in the event of a overrun in double-buffer mode.
    match unsafe {
        transfer.next_transfer_with(|buf, _current_buffer| {
            rprintln!("Buf current reads := {:?}", buf);

            let len = buf.len();
            /*
            Fetch any DMA errors that might have occured.
            For some reason these arn't exposed on the transfer interface so we need to fetch them
            directly from the source.

            NOTE(safety): atomic reads with no side effects.
             */
            let direct_mode_error = Stream4::<DMA2>::get_direct_mode_error_flag();
            let transfer_error = Stream4::<DMA2>::get_transfer_error_flag();
            let fifo_error = Stream4::<DMA2>::get_fifo_error_flag();
            if direct_mode_error || transfer_error || fifo_error {
                rprintln!(
                    "DMA transfer error occured! direct mode:={},transfer:={},fifo:={}",
                    direct_mode_error,
                    transfer_error,
                    fifo_error
                );
            };
            (buf, len)
        })
    } {
        Ok(x) => {
            rprintln!("successfully reconfigured RX DMA, x = {}", x)
        }
        Err(err) => {
            rprintln!("DMA error occured RX DMA reconfigure. e:={:?}", err)
        }
    }

    transfer.clear_interrupts();
    unsafe { clear_idle_interrupt() };
}