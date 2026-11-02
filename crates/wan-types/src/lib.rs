mod error;
mod registry;
mod version;

pub use error::{
    ApiError, ApiErrorBuildError, ApiErrorData, ERROR_REGISTRY, ErrorDefinition, ErrorDomain,
    ErrorKind, JSON_RPC_CANCELLED, JSON_RPC_INTERNAL_ERROR, JSON_RPC_INVALID_PARAMS,
    JSON_RPC_INVALID_REQUEST, JSON_RPC_METHOD_NOT_FOUND, JSON_RPC_PARSE_ERROR, WAN_ERROR_END,
    WAN_ERROR_START, WAN_RESERVED_ERROR_END, WAN_RESERVED_ERROR_START, is_reserved_wan_error_code,
    is_wan_error_code,
};
pub use registry::{
    FIXED_DOMAINS, METHOD_REGISTRY, Method, NOTIFICATION_REGISTRY, Notification, RegistryError,
    is_protocol_message_name, is_valid_method_name, is_valid_notification_name, validate_registry,
};
pub use version::{ProtocolVersion, ProtocolVersionError};
