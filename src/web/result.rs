use crate::web::error::Error;

/// Result type which can be used with the `?` operator in routes.
pub type Result<T = ()> = std::result::Result<T, Error>;
