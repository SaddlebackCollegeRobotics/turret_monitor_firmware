# RTIC
Real Time For the Masses (RTFM), AKA Real-Time Interrupt-driven Concurrency (RTIC)
is a framework for doing task-based concurrency on microcontrollers.
Its basic function is to provide a [RTOS](https://en.wikipedia.org/wiki/Real-time_operating_system).

This project uses RTIC since it makes programming with interrupts much easier and safer than
manually handling the tasks. It provides the means for registering interrupt handlers, as well
as communication mechanisms for exchanging data between tasks safely.