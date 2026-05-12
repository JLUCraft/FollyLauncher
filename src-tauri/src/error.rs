use serde::Serialize;




#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherError {

    pub code: String,

    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl LauncherError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(
        code: impl Into<String>,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: Some(detail.into()),
        }
    }

    #[cfg(test)]
    pub fn contains(&self, needle: &str) -> bool {
        self.code.contains(needle)
            || self.message.contains(needle)
            || self
                .detail
                .as_ref()
                .is_some_and(|detail| detail.contains(needle))
    }
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for LauncherError {}


impl From<LauncherError> for String {
    fn from(e: LauncherError) -> Self {
        e.message
    }
}

impl From<&str> for LauncherError {
    fn from(msg: &str) -> Self {
        Self::new("ERROR", msg.to_string())
    }
}

impl From<String> for LauncherError {
    fn from(msg: String) -> Self {
        Self::new("ERROR", msg)
    }
}

impl From<anyhow::Error> for LauncherError {
    fn from(e: anyhow::Error) -> Self {
        Self::new("INTERNAL_ERROR", e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_error_serializes_camel_case() {
        let err = LauncherError::with_detail("NOT_FOUND", "实例未找到", "instance-123");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"detail\""));
        assert!(json.contains("NOT_FOUND"));
    }

    #[test]
    fn launcher_error_display_includes_code_and_message() {
        let err = LauncherError::new("CONFIG_INVALID", "配置无效");
        let display = err.to_string();
        assert!(display.contains("CONFIG_INVALID"));
        assert!(display.contains("配置无效"));
    }
}
