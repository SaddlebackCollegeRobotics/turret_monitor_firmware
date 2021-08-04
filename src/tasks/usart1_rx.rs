use crate::app::{on_usart1_rxne, Usart1TransferRx};
use crate::tasks::TxBufferState;
use rtt_target::rprintln;
use stm32f4xx_hal::dma::DMAError;

pub(crate) fn on_usart1_rxne(ctx: on_usart1_rxne::Context) {
    rprintln!("Packet Recevied!");
    let mut transfer: &mut Usart1TransferRx = ctx.local.recv;

    // NOTE(unsafe): only unsafe in the event of a overrun in double-buffer mode.
    match unsafe { transfer.next_transfer_with(|buf, _current_buffer| {
        rprintln!("Buf current reads := {:?}", buf);

        let len = buf.len();
        (buf, len)
    })
    } {
        Ok(x) => {
            rprintln!("successfully reconfigured RX DMA, x = {}", x)
        }
        Err(err) => {
            rprintln!("DMA error occured during RX. e:={:?}", err)
        }
    }

    transfer.clear_interrupts();
}