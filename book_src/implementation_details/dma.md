# DMA

DMA, or `Direct Memory Access` is a feature of modern procecessors.

To understand what it does, lets first describe what it replaces.
Take, for example, a UART.

The default interface for a uart only allows the programmer to write one byte at a time, in a **blocking** method.
```rs
block!(uart.write(0xFFu8)).expect("failed to write byte!");
```
If the developer wants to write an *entire* buffer of bytes, they have to do it one byte at a time.
```rs
let buf = [0u8;32]

for byte in buf {
    block!(uart.write(byte)).expect("failed to write byte");
}
```

This process is **extremely** inefficient, slow, and requires the device's CPU to be constantly processing these bytes.
This spends cycles the processor could be spending doing more interesting and important tasks.

For reference, the maximum practical speed a UART can be run at using CPU management is `9600 baud`.
For faster speeds, such as `115200`, DMA is required.
## So what does DMA do?
Modern devices have one or more DMA coprocessors, which act independently of the primary CPU.
This coprocessor is also known as a `DMA Controller`.
These DMA controllers allow the device to asynchronously move memory around, such as with `Peripheral to memory`, `memory to peripheral`, and `memory to memory` modes.

Effectively, once a DMA request is started the primary CPU can forget about it and go do something else.

On the `STM32F446`(herein `f4`), the chip this project targets, there are two DMA controllers, `DMA1` and `DMA2`.

Each DMA Controller on f4 commands 8 `streams`, meaning each controller can handle up to 8 concurrent DMA requests.
Each `stream` is mapped to a specific set of peripherals on the f4, known as a `channel`.
Consult the reference manual to determine which DMA, stream, and channel the desired peripheral is on.

### Requests and Transactions
In terms of abstraction, the core unit of DMA is the `Transaction`.

A `transaction` represents a specific DMA request, such as `memory to memory` transfers.

Every transaction requires a several components:
 - the kind of transaction
 - ownership to the source and destination buffers
   - These buffers MUST outlive the DMA request itself, or UB occurs.
 - In the case of `Peripheral to memory` or `memory to peripheral` transactions, the peripheral has to be configured for DMA.
 - Any special modes or behaviors of the transaction (such as `burst` or `double-buffer` modes.)

### DMA interrupts
Since DMA acts asynchronously of the main CPU, a signalling mechanism is required to
*notify* the CPU when DMA finishes servicing a request.
Thankfully, the CPU already has such a mechanism: interrupts.

DMA transactions can be configured to fire interrupts when certain events happen such as 
when the transfer completes, or an error occured during request servicing.
