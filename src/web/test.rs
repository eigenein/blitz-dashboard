use poem::test::TestClient;
use poem::{Endpoint, EndpointExt};
use sentry::ClientInitGuard;

use crate::prelude::Result;
use crate::web::create_standalone_app;
use crate::web::tracking_code::TrackingCode;

pub async fn create_standalone_test_client() -> Result<(ClientInitGuard, TestClient<impl Endpoint>)>
{
    let sentry_guard = crate::tracing::init(None, 0.0)?;
    let app = create_standalone_app().await?.data(TrackingCode::default());
    Ok((sentry_guard, TestClient::new(app)))
}
