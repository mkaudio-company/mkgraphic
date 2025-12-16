//! Payload type for drag-and-drop data.

use std::collections::HashMap;

/// A payload containing MIME-typed data for drag-and-drop operations.
#[derive(Debug, Clone, Default)]
pub struct Payload {
    data: HashMap<String, String>,
}

impl Payload {
    /// Creates a new empty payload.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Inserts data with the given MIME type.
    pub fn insert(&mut self, mime_type: impl Into<String>, data: impl Into<String>) {
        self.data.insert(mime_type.into(), data.into());
    }

    /// Gets data for the given MIME type.
    pub fn get(&self, mime_type: &str) -> Option<&String> {
        self.data.get(mime_type)
    }

    /// Returns true if the payload contains data for the given MIME type.
    pub fn contains(&self, mime_type: &str) -> bool {
        self.data.contains_key(mime_type)
    }

    /// Returns true if the payload is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of MIME types in the payload.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns an iterator over the MIME types.
    pub fn mime_types(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    /// Clears the payload.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl std::ops::Index<&str> for Payload {
    type Output = String;

    fn index(&self, mime_type: &str) -> &Self::Output {
        self.data.get(mime_type).expect("MIME type not found")
    }
}

/// Common MIME types.
pub mod mime_types {
    pub const TEXT_PLAIN: &str = "text/plain";
    pub const TEXT_URI_LIST: &str = "text/uri-list";
    pub const TEXT_HTML: &str = "text/html";
    pub const APPLICATION_JSON: &str = "application/json";
    pub const IMAGE_PNG: &str = "image/png";
    pub const IMAGE_JPEG: &str = "image/jpeg";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload() {
        let mut payload = Payload::new();
        payload.insert(mime_types::TEXT_PLAIN, "Hello, World!");
        payload.insert(mime_types::TEXT_URI_LIST, "file:///path/to/file.txt");

        assert!(payload.contains(mime_types::TEXT_PLAIN));
        assert_eq!(payload.get(mime_types::TEXT_PLAIN), Some(&"Hello, World!".to_string()));
        assert_eq!(payload.len(), 2);
    }
}
