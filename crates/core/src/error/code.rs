#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ErrorCode {
    // Client-side errors (10000–10999)
    InvalidParameter = 10000,
    MissingConfiguration = 10020,
    Incompatible = 10030,
    PayloadTooLarge = 10070,
    RequestTimeout = 10080,
    MethodNotAllowed = 10090,

    // Authentication and authorization errors (20000–20999)
    PermissionDenied = 20000,
    AccountDisabled = 20010,
    Forbidden = 20020,
    OAuth2ItemDisabled = 20050,
    MissingRefreshToken = 20060,

    // Resource errors (30000–30999)
    ResourceNotFound = 30000,
    TooManyRequest = 30020,
    AlreadyExists = 30030,

    // Network connection errors (40000–40999)
    NetworkError = 40000,
    ConnectionTimeout = 40010,
    ConnectionPoolTimeout = 40020,
    HttpResponseError = 40030,

    // Mail service errors (50000–50999)
    ImapCommandFailed = 50000,
    ImapAuthenticationFailed = 50010,
    ImapUnexpectedResult = 50020,
    AutoconfigFetchFailed = 50060,
    // Internal system errors (70000–70999)
    InternalError = 70000,
    UnhandledPoemError = 70010,
}

impl ErrorCode {
    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
}
