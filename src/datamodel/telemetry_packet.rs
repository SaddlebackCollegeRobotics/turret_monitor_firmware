use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TurretTelemetryPacket {
    pub turret_pos: f32,
}
