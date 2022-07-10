/// Clears current user in Sentry.
pub fn clear_user() {
    sentry::configure_scope(|scope| scope.set_user(None));
}
