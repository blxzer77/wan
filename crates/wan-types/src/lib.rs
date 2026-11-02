mod registry;
mod version;

pub use registry::{
    FIXED_DOMAINS, METHOD_REGISTRY, Method, NOTIFICATION_REGISTRY, Notification, RegistryError,
    is_protocol_message_name, is_valid_method_name, is_valid_notification_name, validate_registry,
};
pub use version::{ProtocolVersion, ProtocolVersionError};
