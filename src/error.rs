use actix_http::ResponseBuilder;
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("{{'sqlx': '{0:?}'}}")]
    Sqlx(#[from] sqlx::Error),
    // #[error("{{'error': 'None'}}")]
    // None,
    // #[error("{{'error': 'Unauthorized'}}")]
    // Unauthorized,
    // #[error("{{'error': '{0}'}}")]
    // S(String),
    // #[error("{{'sserror': '{0}', 'data': '{1}'}}")]
    // SS(String, String),
    // // #[error("{{'error': '{1}', 'context': '{0}' }}")]
    // // SQL(String, diesel::result::Error),
    // #[error("{{'error': '{1:?}', 'sql': '{0}'}}")]
    // SQLXS(String, db::DbError),
    // #[error("{{'error': '{0:?}'}}")]
    // SQLX(#[from] db::DbError),
    // // #[error("{{'error': '{1}', 'context': '{0}' }}")]
    // // Conn(String, diesel::result::ConnectionError),
    // #[error(r#"{{'mgmt': '{0:?}'}}"#)]
    // Mgmt(#[from] mgmt::Er),
    // /* #[error("{{'error': 'NoneError'}}")]
    //  * NoneError, */
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": self.to_string()}))
    }
}

// impl From<actix_web::error::BlockingError<ApiError>> for ApiError {
//     fn from(error: actix_web::error::BlockingError<ApiError>) -> ApiError {
//         use actix_web::error::BlockingError;
//         match error {
//             BlockingError::Error(e) => e,
//             BlockingError::Canceled => ApiError::S("blocking thread cancelled".to_string()),
//         }
//     }
// }
// impl From<&str> for ApiError {
//     fn from(error: &str) -> ApiError {
//         ApiError::S(error.to_string())
//     }
// }
