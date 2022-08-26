use crate::wargaming;

#[derive(Copy, Clone)]
pub struct VehicleAttributes {
    pub tank_id: wargaming::TankId,
    pub victory_ratio: f64,
    pub magnitude: f64,
}
