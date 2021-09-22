use crate::app::{tim8_cc, QeiMonitor};
use rtic::mutex_prelude::*;
use stm32f4xx_hal::hal::Qei;

pub(crate) fn tim8_cc(mut context: tim8_cc::Context) {
    let monitor: &QeiMonitor = &context.local.monitor;

    // observe duty cycle
    // This is done up here to minimize time in the critical section.
    let observation = monitor.count();

    // entering critical section
    context.shared.last_observed_turret_position.lock(|guard| {
        // update the shared state
        *guard = observation;
    });
    // leaving critical section
}
