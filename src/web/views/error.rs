use poem::handler;

use crate::prelude::*;

#[handler]
pub async fn get_error() -> Result<()> {
    bail!("simulated error")
}
