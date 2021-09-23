use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TurretDirection {
    Forward,
    Backward,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TurretTelemetryPacket {
    pub turret_pos: u32,
    pub turret_rot: TurretDirection,
}
