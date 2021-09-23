use anyhow::anyhow;
use phf::{phf_map, Map};

#[allow(dead_code)]
pub fn to_client_id(tank_id: i32) -> crate::Result<i32> {
    const COMPONENT_VEHICLE: i32 = 1;
    debug_assert_eq!(tank_id & COMPONENT_VEHICLE, COMPONENT_VEHICLE);

    let nation = (tank_id >> 4) & 0xF;
    match NATION_IDS.get(&nation) {
        Some(nation_id) => Ok(nation_id + (tank_id >> 8)),
        None => Err(anyhow!("unexpected nation {} for tank {}", nation, tank_id)),
    }
}

#[allow(dead_code)]
static NATION_IDS: Map<i32, i32> = phf_map! {
    0_i32 => 20000, // USSR
    1_i32 => 30000, // Germany
    2_i32 => 10000, // USA
    3_i32 => 60000, // China
    4_i32 => 70000, // France
    5_i32 => 40000, // UK
    6_i32 => 50000, // Japan
    7_i32 => 100000, // Other
    8_i32 => 80000, // Europe
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_client_id_ok() -> crate::Result {
        assert_eq!(to_client_id(2817)?, 20011); // USSR
        assert_eq!(to_client_id(54289)?, 30212); // Germany
        assert_eq!(to_client_id(52257)?, 10204); // USA
        assert_eq!(to_client_id(9009)?, 60035); // China
        assert_eq!(to_client_id(18257)?, 40071); // UK
        assert_eq!(to_client_id(5953)?, 70023); // France
        assert_eq!(to_client_id(4193)?, 50016); // Japan
        assert_eq!(to_client_id(5489)?, 100021); // Other
        assert_eq!(to_client_id(1409)?, 80005); // Europe
        Ok(())
    }
}
