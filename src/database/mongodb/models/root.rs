use serde::Deserialize;

#[derive(Deserialize)]
pub struct Root<T> {
    #[serde(rename = "root")]
    pub root: T,
}
