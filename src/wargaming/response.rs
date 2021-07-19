use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Response<T> {
    Data { data: T },
    Error { error: Error },
}

#[derive(Deserialize, Debug)]
pub struct Error {
    #[serde(default)]
    pub message: Message,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum Message {
    #[serde(rename = "NOT_ENOUGH_SEARCH_LENGTH")]
    NotEnoughSearchLength,

    #[serde(rename = "REQUEST_LIMIT_EXCEEDED")]
    RequestLimitExceeded,

    #[serde(rename = "APPLICATION_IS_BLOCKED")]
    ApplicationIsBlocked,

    #[serde(rename = "INVALID_APPLICATION_ID")]
    InvalidApplicationId,

    #[serde(rename = "INVALID_IP_ADDRESS")]
    InvalidIpAddress,

    #[serde(rename = "SEARCH_NOT_SPECIFIED")]
    SearchNotSpecified,

    #[serde(rename = "ACCOUNT_ID_LIST_LIMIT_EXCEEDED")]
    AccountIdListLimitExceeded,

    #[serde(other)]
    Other,
}

impl Default for Message {
    fn default() -> Self {
        Self::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_result_ok() -> crate::Result {
        let response = serde_json::from_str::<Response<i32>>(
            // language=JSON
            r#"{"data": 42}"#,
        )?;
        match response {
            Response::Data { data } => assert_eq!(data, 42),
            Response::Error { .. } => unreachable!(),
        }
        Ok(())
    }

    #[test]
    fn parse_known_error_ok() -> crate::Result {
        let response = serde_json::from_str::<Response<i32>>(
            // language=JSON
            r#"{"status":"error","error":{"field":"search","message":"NOT_ENOUGH_SEARCH_LENGTH","code":407,"value":"a"}}"#,
        )?;
        match response {
            Response::Data { .. } => unreachable!(),
            Response::Error { error } => {
                assert_eq!(error.message, Message::NotEnoughSearchLength)
            }
        }
        Ok(())
    }

    #[test]
    fn parse_unknown_error_ok() -> crate::Result {
        let response = serde_json::from_str::<Response<i32>>(
            // language=JSON
            r#"{"status":"error","error":{"message":"WTF"}}"#,
        )?;
        match response {
            Response::Error { error } => assert_eq!(error.message, Message::Other),
            Response::Data { .. } => unreachable!(),
        }
        Ok(())
    }

    #[test]
    fn parse_missing_error_message_ok() -> crate::Result {
        let response = serde_json::from_str::<Response<i32>>(
            // language=JSON
            r#"{"status":"error","error":{}}"#,
        )?;
        match response {
            Response::Error { error } => assert_eq!(error.message, Message::Other),
            Response::Data { .. } => unreachable!(),
        }
        Ok(())
    }
}
