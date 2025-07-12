//! Domain types for the accounts module

use std::fmt;
use std::ops::Deref;

/// Account identifier
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

/// Model code identifier
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

/// Contract identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractId(pub i32);

impl ContractId {
    /// Get the inner value
    pub fn value(&self) -> i32 {
        self.0
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for ContractId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

// pub struct ModelCode(pub String);

/// Account group for filtering
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
        Self(s.to_string())
    }
}
