use anyhow::anyhow;

use crate::models::Nation;

pub fn get_nation(tank_id: i32) -> crate::Result<Nation> {
    const NATIONS: &[Nation] = &[
        Nation::Ussr,
        Nation::Germany,
        Nation::Usa,
        Nation::China,
        Nation::France,
        Nation::Uk,
        Nation::Japan,
        Nation::Other,
        Nation::Europe,
    ];

    const COMPONENT_VEHICLE: i32 = 1;
    debug_assert_eq!(tank_id & COMPONENT_VEHICLE, COMPONENT_VEHICLE);

    let nation = ((tank_id >> 4) & 0xF) as usize;
    NATIONS
        .get(nation)
        .copied()
        .ok_or_else(|| anyhow!("unexpected nation {} for tank {}", nation, tank_id))
}

pub fn to_client_id(tank_id: i32) -> crate::Result<i32> {
    let nation_id = match get_nation(tank_id)? {
        Nation::Ussr => 20000,
        Nation::Germany => 30000,
        Nation::Usa => 10000,
        Nation::China => 60000,
        Nation::France => 70000,
        Nation::Uk => 40000,
        Nation::Japan => 50000,
        Nation::Other => 100000,
        Nation::Europe => 80000,
    };
    Ok(nation_id + (tank_id >> 8))
}

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
