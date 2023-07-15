use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, strum_macros::AsRefStr)]
#[serde(tag = "type", content = "data")]
pub enum Error {
    // Auth errors
    AuthExpired,
    AuthFailCtxNotInRequestExt,
    InvalidAuthHeader,
    InvalidCredentials,
    InvalidToken,
    JWTTokenCreationError,
    JWTValidationError,
    MissingAuth,
    PasswordConfirmMismatch,
    UsernameExists,

    // Server errors
    ClearLinksFail,
    CreateLinkFail,
    DeleteTokenFail,
    GetLinksFail,
    GetUsersFail,
    GetTokensFail,
    InvalidDeleteToken,
    SignInFail,
    SignUpFail,
    CtxCreationFail,
    MissingEnvVar,
    GenTokenFail,
    SplitUserIdFail,
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("->> {:<12} - {self:?}", "INTO_RES");

        // Create a placeholder Axum response.
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

        // Insert the Error into the response.
        response.extensions_mut().insert(self);

        response
    }
}

impl Error {
    pub fn client_status_and_error(&self) -> (StatusCode, ClientError) {
        match self {
            Self::AuthExpired => (StatusCode::UNAUTHORIZED, ClientError::AUTH_EXPIRED),
            Self::AuthFailCtxNotInRequestExt | Self::MissingAuth => {
                (StatusCode::UNAUTHORIZED, ClientError::NO_AUTH)
            }
            Self::UsernameExists => (StatusCode::BAD_REQUEST, ClientError::USERNAME_EXISTS),
            Self::JWTValidationError
            | Self::PasswordConfirmMismatch
            | Self::InvalidAuthHeader
            | Self::InvalidToken
            | Self::InvalidCredentials
            | Self::InvalidDeleteToken
            | Self::GenTokenFail => (StatusCode::BAD_REQUEST, ClientError::INVALID_AUTH),
            Self::ClearLinksFail
            | Self::CreateLinkFail
            | Self::DeleteTokenFail
            | Self::GetLinksFail
            | Self::GetUsersFail
            | Self::GetTokensFail
            | Self::JWTTokenCreationError
            | Self::SignInFail
            | Self::SignUpFail
            | Self::CtxCreationFail
            | Self::MissingEnvVar
            | Self::SplitUserIdFail => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ClientError::SERVICE_ERROR,
            ),
        }
    }
}

#[derive(Debug, strum_macros::AsRefStr)]
#[allow(non_camel_case_types)]
pub enum ClientError {
    LOGIN_FAILED,
    NO_AUTH,
    INVALID_AUTH,
    AUTH_EXPIRED,
    SERVICE_ERROR,
    USERNAME_EXISTS,
}
