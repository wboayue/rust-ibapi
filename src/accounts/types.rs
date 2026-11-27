//! Domain types for the accounts module

use std::fmt;
use std::ops::Deref;

/// Account identifier
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(pub String);

impl Deref for AccountId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AccountId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AccountId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<&String> for AccountId {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<&AccountId> for AccountId {
    fn from(id: &AccountId) -> Self {
        id.clone()
    }
}

impl From<AccountId> for String {
    fn from(id: AccountId) -> Self {
        id.0
    }
}

/// Model code identifier
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelCode(pub String);

impl Deref for ModelCode {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ModelCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ModelCode {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ModelCode {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<&String> for ModelCode {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<&ModelCode> for ModelCode {
    fn from(code: &ModelCode) -> Self {
        code.clone()
    }
}

impl From<ModelCode> for String {
    fn from(code: ModelCode) -> Self {
        code.0
    }
}

/// Account group for filtering
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountGroup(pub String);

impl AccountGroup {
    /// Convert to string representation for API calls
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for AccountGroup {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for AccountGroup {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
