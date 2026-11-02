use std::collections::BTreeMap;

pub const JSON_RPC_PARSE_ERROR: i32 = -32700;
pub const JSON_RPC_INVALID_REQUEST: i32 = -32600;
pub const JSON_RPC_METHOD_NOT_FOUND: i32 = -32601;
pub const JSON_RPC_INVALID_PARAMS: i32 = -32602;
pub const JSON_RPC_INTERNAL_ERROR: i32 = -32603;
pub const JSON_RPC_CANCELLED: i32 = -32800;

pub const WAN_ERROR_START: i32 = -32_089;
pub const WAN_ERROR_END: i32 = -32_000;
pub const WAN_RESERVED_ERROR_START: i32 = -32_099;
pub const WAN_RESERVED_ERROR_END: i32 = -32_090;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ErrorDomain {
    Routine,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ErrorKind {
    RoutineRateLimited,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct ErrorDefinition {
    pub kind: ErrorKind,
    pub name: &'static str,
    pub domain: ErrorDomain,
    pub code: i32,
}

pub const ERROR_REGISTRY: &[ErrorDefinition] = &[ErrorDefinition::new(
    ErrorKind::RoutineRateLimited,
    "routine_rate_limited",
    ErrorDomain::Routine,
    -32_020,
)];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiError {
    pub code: i32,
    pub data: ApiErrorData,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiErrorData {
    pub kind: ErrorKind,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub params: BTreeMap<String, String>,
    pub request_id: Option<String>,
}

impl ErrorDefinition {
    pub const fn new(kind: ErrorKind, name: &'static str, domain: ErrorDomain, code: i32) -> Self {
        Self {
            kind,
            name,
            domain,
            code,
        }
    }
}

impl ErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RoutineRateLimited => "routine_rate_limited",
        }
    }

    pub const fn domain(self) -> ErrorDomain {
        match self {
            Self::RoutineRateLimited => ErrorDomain::Routine,
        }
    }

    pub const fn code(self) -> i32 {
        match self {
            Self::RoutineRateLimited => -32_020,
        }
    }

    pub const fn allowed_params(self) -> &'static [&'static str] {
        match self {
            Self::RoutineRateLimited => &[],
        }
    }
}

impl ErrorDomain {
    pub const fn code_start(self) -> i32 {
        match self {
            Self::Routine => -32_029,
        }
    }

    pub const fn code_end(self) -> i32 {
        match self {
            Self::Routine => -32_020,
        }
    }

    pub const fn contains(self, code: i32) -> bool {
        code >= self.code_start() && code <= self.code_end()
    }
}

impl ApiError {
    pub fn new(
        kind: ErrorKind,
        retryable: bool,
        retry_after_ms: Option<u64>,
        params: BTreeMap<String, String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            code: kind.code(),
            data: ApiErrorData {
                kind,
                retryable,
                retry_after_ms,
                params,
                request_id,
            },
        }
    }
}

pub const fn is_wan_error_code(code: i32) -> bool {
    code >= WAN_ERROR_START && code <= WAN_ERROR_END
}

pub const fn is_reserved_wan_error_code(code: i32) -> bool {
    code >= WAN_RESERVED_ERROR_START && code <= WAN_RESERVED_ERROR_END
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        ApiError, ERROR_REGISTRY, ErrorKind, JSON_RPC_CANCELLED, is_reserved_wan_error_code,
        is_wan_error_code,
    };

    #[test]
    fn error_registry_is_unique_and_in_domain() {
        for (index, definition) in ERROR_REGISTRY.iter().enumerate() {
            assert_eq!(definition.name, definition.kind.as_str());
            assert_eq!(definition.domain, definition.kind.domain());
            assert_eq!(definition.code, definition.kind.code());
            assert!(definition.domain.contains(definition.code));
            assert!(
                ERROR_REGISTRY[index + 1..]
                    .iter()
                    .all(|other| other.name != definition.name && other.code != definition.code)
            );
        }
    }

    #[test]
    fn validates_standard_and_wan_code_ranges() {
        assert_eq!(JSON_RPC_CANCELLED, -32800);
        assert!(is_wan_error_code(-32_000));
        assert!(is_wan_error_code(-32_089));
        assert!(!is_wan_error_code(-32_090));
        assert!(is_reserved_wan_error_code(-32_090));
        assert!(is_reserved_wan_error_code(-32_099));
    }

    #[test]
    fn builds_api_error_with_protocol_fields() {
        let error = ApiError::new(
            ErrorKind::RoutineRateLimited,
            true,
            Some(1_000),
            BTreeMap::new(),
            Some("req-1".to_owned()),
        );
        assert_eq!(error.code, -32_020);
        assert_eq!(error.data.kind.as_str(), "routine_rate_limited");
        assert!(error.data.retryable);
    }

    #[test]
    fn error_registry_snapshot() {
        insta::assert_debug_snapshot!(ERROR_REGISTRY);
    }
}
