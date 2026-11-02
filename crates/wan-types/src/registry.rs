pub const FIXED_DOMAINS: &[&str] = &[
    "session", "memory", "routine", "vault", "pages", "computer", "acp", "notify", "device",
    "settings", "system",
];

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Method {
    name: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Notification {
    name: &'static str,
}

pub const METHOD_REGISTRY: &[Method] = &[
    Method::new("session/respond_card"),
    Method::new("session/start_turn"),
    Method::new("session/start_upload"),
    Method::new("session/finish_upload"),
    Method::new("device/sync"),
    Method::new("settings/get"),
    Method::new("settings/update"),
    Method::new("settings/reset"),
    Method::new("system/initialize"),
];

pub const NOTIFICATION_REGISTRY: &[Notification] = &[
    Notification::new("session/event/item_updated"),
    Notification::new("system/event/status_changed"),
];

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum RegistryError {
    InvalidMethod(&'static str),
    InvalidNotification(&'static str),
    DuplicateMethod(&'static str),
    DuplicateNotification(&'static str),
}

impl Method {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

impl Notification {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

pub fn is_valid_method_name(name: &str) -> bool {
    let Some((domain, action)) = name.split_once('/') else {
        return false;
    };
    domain_is_fixed(domain) && is_snake_case(action) && !action.contains('/')
}

pub fn is_valid_notification_name(name: &str) -> bool {
    let mut parts = name.split('/');
    let Some(domain) = parts.next() else {
        return false;
    };
    let Some(event) = parts.next() else {
        return false;
    };
    let Some(action) = parts.next() else {
        return false;
    };
    parts.next().is_none() && domain_is_fixed(domain) && event == "event" && is_snake_case(action)
}

pub fn is_protocol_message_name(name: &str) -> bool {
    name.strip_prefix("$/").is_some_and(is_snake_case)
}

pub fn validate_registry() -> Result<(), RegistryError> {
    for method in METHOD_REGISTRY {
        if !is_valid_method_name(method.name()) {
            return Err(RegistryError::InvalidMethod(method.name()));
        }
    }
    for notification in NOTIFICATION_REGISTRY {
        if !is_valid_notification_name(notification.name()) {
            return Err(RegistryError::InvalidNotification(notification.name()));
        }
    }
    for (index, method) in METHOD_REGISTRY.iter().enumerate() {
        if METHOD_REGISTRY[index + 1..]
            .iter()
            .any(|other| other.name() == method.name())
        {
            return Err(RegistryError::DuplicateMethod(method.name()));
        }
    }
    for (index, notification) in NOTIFICATION_REGISTRY.iter().enumerate() {
        if NOTIFICATION_REGISTRY[index + 1..]
            .iter()
            .any(|other| other.name() == notification.name())
        {
            return Err(RegistryError::DuplicateNotification(notification.name()));
        }
    }
    Ok(())
}

fn domain_is_fixed(domain: &str) -> bool {
    FIXED_DOMAINS.contains(&domain)
}

fn is_snake_case(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() || bytes[bytes.len() - 1] == b'_' {
        return false;
    }
    let mut previous_was_underscore = false;
    for byte in bytes {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_was_underscore = false;
        } else if *byte == b'_' && !previous_was_underscore {
            previous_was_underscore = true;
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{
        FIXED_DOMAINS, METHOD_REGISTRY, NOTIFICATION_REGISTRY, is_protocol_message_name,
        is_valid_method_name, is_valid_notification_name, validate_registry,
    };

    #[test]
    fn registry_is_valid_and_unique() {
        validate_registry().expect("registry should follow protocol naming rules");
    }

    #[test]
    fn registry_contains_updated_protocol_names() {
        assert!(
            METHOD_REGISTRY
                .iter()
                .any(|method| method.name() == "session/start_turn")
        );
        assert!(FIXED_DOMAINS.contains(&"settings"));
        assert!(
            NOTIFICATION_REGISTRY
                .iter()
                .any(|notification| notification.name() == "system/event/status_changed")
        );
    }

    #[test]
    fn validates_method_and_notification_shapes() {
        assert!(is_valid_method_name("settings/update"));
        assert!(!is_valid_method_name("turn/start"));
        assert!(is_valid_notification_name("session/event/item_updated"));
        assert!(!is_valid_notification_name("session/item/updated"));
        assert!(is_protocol_message_name("$/cancel_request"));
        assert!(!is_protocol_message_name("$/cancelRequest"));
    }
}
