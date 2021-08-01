# Communication interface
Describes how to interpret data emitted from this firmware.

The primary communication interface to this firmware is via the `USART1` device.

# Packet structure
The maximum length of any packet is defined as
```rs
{{#include ../src/main.rs:70}}
```

Any packet received by this device exceeding this length will be ignored.

- All packets are be COBS encoded
- All packets terminate with the `\x00` (null) sentinel.
- Any bytes after the null terminator in a given transmission are reserved.
- All packets may have up to `BUF_SIZE-5` bytes of data
- the remaining 4 bytes are a `Big Endian` encoded `u32` CRC-32(Ethernet) checksum, followed by the null terminator.
```
| <data> (up to BUF_SIZE-5 bytes) | 4 byte CRC | \x00 |
```

## Details on the CRC-32 checksum
This device utilizes the CRC peripheral to perform the calculation.

Given that the CRC peripheral functions on u32 `word`s, only full words of the payload are 
included in the checksum.

This means, if the payload size is 18, **only the first 16 bytes** (4 Big Endian words) will be fed to the CRC peripheral.


# Status response structure
The payload of a status response is a json-encoded object representing the current device 
observations.