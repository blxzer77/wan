#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Language {
    En,
    ZhCn,
}

impl Language {
    pub const fn locale(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }
}

pub fn negotiate_language(requested: Option<&str>) -> Language {
    match requested {
        Some(value) if starts_with_zh(value) => Language::ZhCn,
        _ => Language::En,
    }
}

fn starts_with_zh(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2
        && (bytes[0] == b'z' || bytes[0] == b'Z')
        && (bytes[1] == b'h' || bytes[1] == b'H')
}

#[cfg(test)]
mod tests {
    use super::{Language, negotiate_language};

    #[test]
    fn maps_zh_language_tags_to_zh_cn() {
        assert_eq!(negotiate_language(Some("zh")), Language::ZhCn);
        assert_eq!(negotiate_language(Some("zh-Hans-CN")), Language::ZhCn);
        assert_eq!(negotiate_language(Some("ZH-tw")).locale(), "zh-CN");
    }

    #[test]
    fn falls_back_to_english_for_other_or_missing_tags() {
        assert_eq!(negotiate_language(Some("en-US")), Language::En);
        assert_eq!(negotiate_language(Some("ja")), Language::En);
        assert_eq!(negotiate_language(None).locale(), "en");
    }
}
