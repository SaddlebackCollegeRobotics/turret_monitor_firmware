# Synopsis
This package provides a STM32 rust firmware packge for monitoring the position of 
valkyrie's turret.

The function of this firmware is to monitor the duty cycle of a PWM input signal,
which comes from a [mag encoder](http://www.ctr-electronics.com/sensors/srx-magnetic-encoder.html)
physical device mechanically mounted to the turret. 

A secondary objective is to measure signals connected to ADC1, to support the science experiment.

