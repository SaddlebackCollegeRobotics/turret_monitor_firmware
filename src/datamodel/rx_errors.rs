
#[derive(Debug)]
pub enum RxError {
    CobsDecoderNeededMoreBytes,
    CobsDecoderError(usize),
    CobsDecoderPushFailed,
    InvalidSenderCrc,
    FailedTelemetrySpawn,
    FailedJsonDeserialize,
    BufferOverflow,
    DmaReconfigFailed,
    DmaTransferFailed,
}