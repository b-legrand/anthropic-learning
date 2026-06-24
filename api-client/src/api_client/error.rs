use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Status { status: u16, body: String },
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Http(e) => write!(f, "http error: {e}"),
            ApiError::Status { status, body } => write!(f, "api returned {status}, {body}"),
        }
    }
}

impl std::error::Error for ApiError {}

// make `?` work: when a reqwest::Error appears in a function returning Result<_, ApiError>
// `?` calls this to convert it
impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Http(e)
    }
}
