#[rocket::get("/error")]
pub async fn get_error() -> crate::web::result::Result {
    Err(anyhow::anyhow!("simulated error").into())
}
