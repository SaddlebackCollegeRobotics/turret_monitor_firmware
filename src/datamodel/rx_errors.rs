#[derive(Debug)]
pub enum RxError {
    CobsDecoderNeededMoreBytes,
    CobsDecoderError(usize),
    CobsDecoderPushFailed,
    InvalidSenderCrc,
    FailedTelemetrySpawn,
    FailedDeserialize,
    BufferOverflow,
    DmaReconfigFailed,
    DmaTransferFailed,
}
