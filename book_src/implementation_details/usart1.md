
# USART1 DMA
[DMA](./dma.md) is used to emit the data from this device to the host.

The `stm32f4xx_hal` only implements the necessary DMA abstractions for the USARTs on the device,
so the UART4 used for previous projects was not usable for this application.

## TX DMA
This device will perodically emit observed telemetry via `USART1_TX`.

Given that the [PWM input interface]() will be constantly firing interrupts, 
it is necessary that the telemetry transmitter runs asynchronously to the main runtime.

- According to RM0390 rev 5, `USART1_TX` is mapped to `DMA2`, Stream 7, channel 4.
- The request kind is `Memory to Peripheral`.
- Given the relatively low period of this task, we configure for `single buffer` mode.
  - This simplifies satisfying safety contracts.
- Since we are writing an entire buffer, DMA is configured to increment the buffer address.
    - (Otherwise it just writes the first byte over and over again.)
- DMA transfer configured to emit an interrupt on request completion.
  - Triggers a bookkeeping task to prevent concurrent DMA requests against the same 
    memory and device.
  - Interrupt handled via the `on_dma2_stream7` task.
  