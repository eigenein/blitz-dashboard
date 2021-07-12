pub mod models;
pub mod view;

pub fn get_account_url(account_id: i32) -> String {
    format!("/ru/{}", account_id)
}
