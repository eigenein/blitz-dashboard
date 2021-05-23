use crate::wargaming::AccountId;
use crate::web::{respond_with_body, State};
use maud::html;
use tide::StatusCode;

pub fn get_user_url(account_id: u32) -> String {
    format!("/ru/{}", account_id)
}

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let _username: AccountId = request.param("user_id")?.parse()?;
    respond_with_body(StatusCode::Ok, html! {})
}
