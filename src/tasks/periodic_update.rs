use rtic::time::duration::Seconds;
use rtt_target::rprintln;
pub(crate) fn periodic_emit_status(context: crate::app::periodic_emit_status::Context) {
    rprintln!("tick!");
    crate::app::periodic_emit_status::spawn_after(Seconds(1u32))
        .expect("failed to re-spawn periodic task.");
}
