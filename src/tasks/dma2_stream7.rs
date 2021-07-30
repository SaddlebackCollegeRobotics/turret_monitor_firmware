use crate::app::on_dma2_stream7;
use crate::tasks::TxBufferState;
use rtt_target::rprintln;

pub(crate) fn on_dma2_stream7(ctx: on_dma2_stream7::Context){
    let dma_state: TxBufferState = ctx.shared.send.take().expect("failed to aquire buffer state");
    match dma_state{
        TxBufferState::Running(tx) => {
            *ctx.shared.send = Some(TxBufferState::Idle(tx))
        }
        TxBufferState::Idle(mut tx) => {
            rprintln!("DMA shouldn't be firing interrupts while we are idle.");
            tx.pause(|f| {});
            *ctx.shared.send = Some(TxBufferState::Idle(tx))
        }
    }
}