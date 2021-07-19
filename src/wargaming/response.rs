use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Response<T> {
    Data { data: T },
    Error { error: ResponseError },
}

#[derive(Deserialize, Debug)]
pub struct ResponseError {
    #[serde(default)]
    pub message: ResponseMessage,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum ResponseMessage {
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

impl Default for ResponseMessage {
    fn default() -> Self {
        Self::Other
    }
}

impl<T> From<Response<T>> for crate::Result<T> {
    fn from(response: Response<T>) -> Self {
        match response {
            Response::Data { data } => Ok(data),
            Response::Error { error } => Err(anyhow::anyhow!("{:?}", error.message)),
        }
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
        match crate::Result::<i32>::from(response) {
            Ok(data) => assert_eq!(data, 42),
            Err(_) => unreachable!(),
        };
        Ok(())
    }

    #[test]
    fn parse_known_error_ok() -> crate::Result {
        let response = serde_json::from_str::<Response<i32>>(
            // language=JSON
            r#"{"status":"error","error":{"field":"search","message":"NOT_ENOUGH_SEARCH_LENGTH","code":407,"value":"a"}}"#,
        )?;
        let result = crate::Result::<i32>::from(response);
        match result {
            Ok(_) => unreachable!(),
            Err(error) => assert_eq!(error.to_string(), "NotEnoughSearchLength"),
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
            Response::Error { error } => assert_eq!(error.message, ResponseMessage::Other),
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
            Response::Error { error } => assert_eq!(error.message, ResponseMessage::Other),
            Response::Data { .. } => unreachable!(),
        }
        Ok(())
    }
}
