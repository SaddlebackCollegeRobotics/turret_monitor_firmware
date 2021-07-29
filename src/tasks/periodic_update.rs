use rtic::time::duration::Seconds;
use rtt_target::rprintln;

use rtic::mutex_prelude::*;

pub(crate) fn periodic_emit_status(
    // unfortunately, as a implementation detail we cannot take an immutable reference
    // to this shared resource since another task mutates it. we have to take it mutably even
    // if we are only reading it.
    mut context: crate::app::periodic_emit_status::Context,
) {
    rprintln!("tick!");

    let mut turret_position: f32 = 0.0;
    /*
        entering critical section
    */
    context.shared.last_observed_turret_position.lock(|guard| {
        turret_position = *guard;
    });
    /*
        leaving critical section
    */

    // re-schedule this task (become periodic)
    crate::app::periodic_emit_status::spawn_after(Seconds(1u32))
        .expect("failed to re-spawn periodic task.");
}
