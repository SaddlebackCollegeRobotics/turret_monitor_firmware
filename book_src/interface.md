# Communication interface
Describes how to interpret data emitted from this firmware.

The primary communication interface to this firmware is via the `USART1` device.

# Packet structure
The maximum length of any packet is defined as
```rs
{{#include ../src/main.rs:71:74}}
```

Any packet received by this device exceeding this length will be ignored.
This device will not emit packets larger this size. 

- All packets are COBS encoded
- All packets terminate with the `\x00` (null) sentinel.
- Any bytes after the sentinel in a given transmission up to the buffer size are reserved.
- All packets may have up to `BUF_SIZE-5` bytes of data
- the trailing 5 bytes are a `Big Endian` encoded `u32` CRC-32(Ethernet) checksum, 
  followed by one or more sentinel bytes.
```
| <data> (up to BUF_SIZE-5 bytes) | 4 byte CRC | \x00 |
```
Example cobs-encoded response packet:
```
b'\x17{"turret_pos":1.0}\x19g\xa0\x85\x00\x00\x00\x00\x00\x00\x00\x00\x00'
```
Decoded it reads as (data, device crc32):
```
({'turret_pos': 1.0}, 426221701)
```

## Details on the CRC-32 checksum
This device utilizes the CRC peripheral to perform the calculation.

Given that the CRC peripheral functions on u32 `word`s, only full words of the payload are 
included in the checksum.

This means, if the payload size is 18, **only the first 16 bytes** (4 Big Endian words) will be fed to the CRC peripheral.


# Status response structure
The payload of a status response is a CBOR-encoded object representing the current device 
observations.

## Request
Requests must be a well-formed packet as defined in [packet structure](#packet-structure).

> Malformed packets will be ignored by the device.

The request object is defined below, though at the time of writing the `kind` field is reserved.
```rs
{{#include ../src/datamodel/request.rs}}
```
### Example request payload
```python
from turret_python_interface.request_packet import RequestPacket
print(bytes(RequestPacket(kind=4)))
# b'\x0c\xa1dkind\x04\n\x10Oo\x01'
print(RequestPacket.from_bytes(b'\x0c\xa1dkind\x04\n\x10Oo\x01'))
# RequestPacket(kind=4)
# 2021-08-08 20:16:36.013 | DEBUG    | turret_python_interface.message_base:from_bytes:24 - data bytes := b'\xa1dkind\x04', device CRC := 168841071
```


## Response
The response will be a well-formed packet.

The response object is defined below.
```rs
{{#include ../src/datamodel/telemetry_packet.rs}}
```

### Example response payload
```python
from turret_python_interface.telemetry_packet import TelemetryPacket
packet = TelemetryPacket(turret_pos=1.0)
print(bytes(packet))
# b'\x10\xa1jturret_pos\xfb?\xf0\x01\x01\x01\x01\x01\x05F/\r\x14\x01'
print(TelemetryPacket.from_bytes(b'\x10\xa1jturret_pos\xfb?\xf0\x01\x01\x01\x01\x01\x05F/\r\x14\x01'))
# 2021-08-08 20:19:40.908 | DEBUG    | turret_python_interface.message_base:from_bytes:24 - data bytes := b'\xa1jturret_pos\xfb?\xf0\x00\x00\x00\x00\x00\x00', device CRC := 1177488660
# TelemetryPacket(turret_pos=1.0)

```