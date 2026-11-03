//! Associated-data construction. Every segment is length-prefixed, so two different field
//! tuples can never produce the same byte string.

pub(crate) struct Aad(Vec<u8>);

impl Aad {
    pub(crate) fn new(domain: &'static str) -> Self {
        let mut aad = Self(Vec::new());
        aad.bytes(domain.as_bytes());
        aad
    }

    pub(crate) fn bytes(&mut self, segment: &[u8]) -> &mut Self {
        self.0
            .extend_from_slice(&(segment.len() as u64).to_be_bytes());
        self.0.extend_from_slice(segment);
        self
    }

    pub(crate) fn text(&mut self, segment: &str) -> &mut Self {
        self.bytes(segment.as_bytes())
    }

    pub(crate) fn number(&mut self, value: u32) -> &mut Self {
        self.bytes(&value.to_be_bytes())
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_prefix_separates_segments() {
        let mut joined = Aad::new("d");
        joined.text("ab").text("c");
        let mut shifted = Aad::new("d");
        shifted.text("a").text("bc");

        assert_ne!(joined.as_slice(), shifted.as_slice());
    }

    #[test]
    fn domain_separates_uses() {
        let mut item = Aad::new("item");
        item.text("x");
        let mut key = Aad::new("key");
        key.text("x");

        assert_ne!(item.as_slice(), key.as_slice());
    }
}
