use crate::models::Nation;

pub type TankId = u16;

/// Converts the API tank ID to the client tank ID.
pub fn to_client_id(tank_id: TankId) -> crate::Result<u32> {
    Ok(Nation::from_tank_id(tank_id)?.get_id() + (tank_id as u32 >> 8))
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
