use crate::app::on_dma2_stream7;
use crate::tasks::TxBufferState;
use rtt_target::rprintln;

pub(crate) fn on_dma2_stream7(ctx: on_dma2_stream7::Context) {
    let dma_state: TxBufferState = ctx
        .shared
        .send
        .take()
        .expect("failed to aquire buffer state");
    match dma_state {
        TxBufferState::Running(mut tx) => {
            // turns out DMA doesn't clean up its own interrupts, so we have to do so ourselves.
            tx.clear_transfer_complete_interrupt();
            // nothing left to transfer, pause DMA.
            tx.pause(|_| {});
            *ctx.shared.send = Some(TxBufferState::Idle(tx));
        }
        TxBufferState::Idle(mut tx) => {
            // this shouldn't happen.
            rprintln!("DMA shouldn't be firing interrupts while we are idle.");
            tx.pause(|_| {});
            *ctx.shared.send = Some(TxBufferState::Idle(tx))
        }
    }
}
