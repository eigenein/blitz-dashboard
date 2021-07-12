use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};

pub struct SecurityHeaders;

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "SecurityHeaders",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.remove_header("Server");
        response.set_raw_header("X-DNS-Prefetch-Control", "on");
        response.set_raw_header("X-Content-Type-Options", "nosniff");
        response.set_raw_header("X-Frame-Options", "deny");
        response.set_raw_header("Strict-Transport-Security", "max-age=5184000");
        response.set_raw_header("X-XSS-Protection", "1; mode=block");
    }
}
