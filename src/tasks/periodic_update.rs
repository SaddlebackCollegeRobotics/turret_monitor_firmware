use rtic::time::duration::Seconds;
pub(crate) fn periodic_emit_status(context: crate::app::periodic_emit_status::Context){
    periodic_emit_status::spawn_after(Seconds(1));
    todo!()
}
