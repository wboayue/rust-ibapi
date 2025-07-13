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
        Self(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod account_id {
        use super::*;

        #[test]
        fn test_new() {
            let id = AccountId("U123456".to_string());
            assert_eq!(id.0, "U123456");
        }

        #[test]
        fn test_deref() {
            let id = AccountId("U123456".to_string());
            assert_eq!(&*id, "U123456");
            assert_eq!(id.len(), 7);
            assert!(id.starts_with("U"));
        }

        #[test]
        fn test_display() {
            let id = AccountId("U123456".to_string());
            assert_eq!(format!("{}", id), "U123456");
        }

        #[test]
        fn test_from_string() {
            let id = AccountId::from("U123456".to_string());
            assert_eq!(id.0, "U123456");
        }

        #[test]
        fn test_from_str() {
            let id = AccountId::from("U123456");
            assert_eq!(id.0, "U123456");
        }

        #[test]
        fn test_empty_account_id() {
            let id = AccountId::from("");
            assert_eq!(id.0, "");
            assert_eq!(id.len(), 0);
        }

        #[test]
        fn test_special_characters() {
            let id = AccountId::from("U-123_456.789");
            assert_eq!(id.0, "U-123_456.789");
        }

        #[test]
        fn test_equality() {
            let id1 = AccountId::from("U123456");
            let id2 = AccountId::from("U123456");
            let id3 = AccountId::from("U654321");
            
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn test_hash() {
            use std::collections::HashSet;
            
            let mut set = HashSet::new();
            set.insert(AccountId::from("U123456"));
            set.insert(AccountId::from("U123456"));
            set.insert(AccountId::from("U654321"));
            
            assert_eq!(set.len(), 2);
            assert!(set.contains(&AccountId::from("U123456")));
            assert!(set.contains(&AccountId::from("U654321")));
        }
    }

    mod model_code {
        use super::*;

        #[test]
        fn test_new() {
            let code = ModelCode("MODEL1".to_string());
            assert_eq!(code.0, "MODEL1");
        }

        #[test]
        fn test_deref() {
            let code = ModelCode("MODEL1".to_string());
            assert_eq!(&*code, "MODEL1");
            assert_eq!(code.len(), 6);
            assert!(code.contains("MODEL"));
        }

        #[test]
        fn test_display() {
            let code = ModelCode("MODEL1".to_string());
            assert_eq!(format!("{}", code), "MODEL1");
        }

        #[test]
        fn test_from_string() {
            let code = ModelCode::from("MODEL1".to_string());
            assert_eq!(code.0, "MODEL1");
        }

        #[test]
        fn test_from_str() {
            let code = ModelCode::from("MODEL1");
            assert_eq!(code.0, "MODEL1");
        }

        #[test]
        fn test_empty_model_code() {
            let code = ModelCode::from("");
            assert_eq!(code.0, "");
            assert!(code.is_empty());
        }

        #[test]
        fn test_equality() {
            let code1 = ModelCode::from("MODEL1");
            let code2 = ModelCode::from("MODEL1");
            let code3 = ModelCode::from("MODEL2");
            
            assert_eq!(code1, code2);
            assert_ne!(code1, code3);
        }

        #[test]
        fn test_hash() {
            use std::collections::HashSet;
            
            let mut set = HashSet::new();
            set.insert(ModelCode::from("MODEL1"));
            set.insert(ModelCode::from("MODEL1"));
            set.insert(ModelCode::from("MODEL2"));
            
            assert_eq!(set.len(), 2);
        }
    }

    mod contract_id {
        use super::*;

        #[test]
        fn test_new() {
            let id = ContractId(12345);
            assert_eq!(id.0, 12345);
        }

        #[test]
        fn test_value() {
            let id = ContractId(12345);
            assert_eq!(id.value(), 12345);
        }

        #[test]
        fn test_display() {
            let id = ContractId(12345);
            assert_eq!(format!("{}", id), "12345");
        }

        #[test]
        fn test_from_i32() {
            let id = ContractId::from(12345);
            assert_eq!(id.0, 12345);
        }

        #[test]
        fn test_zero_contract_id() {
            let id = ContractId(0);
            assert_eq!(id.value(), 0);
            assert_eq!(format!("{}", id), "0");
        }

        #[test]
        fn test_negative_contract_id() {
            let id = ContractId(-1);
            assert_eq!(id.value(), -1);
            assert_eq!(format!("{}", id), "-1");
        }

        #[test]
        fn test_max_contract_id() {
            let id = ContractId(i32::MAX);
            assert_eq!(id.value(), i32::MAX);
            assert_eq!(format!("{}", id), i32::MAX.to_string());
        }

        #[test]
        fn test_equality() {
            let id1 = ContractId(12345);
            let id2 = ContractId(12345);
            let id3 = ContractId(54321);
            
            assert_eq!(id1, id2);
            assert_ne!(id1, id3);
        }

        #[test]
        fn test_copy() {
            let id1 = ContractId(12345);
            let id2 = id1; // Copy
            assert_eq!(id1, id2);
            assert_eq!(id1.value(), 12345);
            assert_eq!(id2.value(), 12345);
        }

        #[test]
        fn test_hash() {
            use std::collections::HashSet;
            
            let mut set = HashSet::new();
            set.insert(ContractId(12345));
            set.insert(ContractId(12345));
            set.insert(ContractId(54321));
            
            assert_eq!(set.len(), 2);
        }
    }

    mod account_group {
        use super::*;

        #[test]
        fn test_new() {
            let group = AccountGroup("All".to_string());
            assert_eq!(group.0, "All");
        }

        #[test]
        fn test_as_str() {
            let group = AccountGroup("All".to_string());
            assert_eq!(group.as_str(), "All");
        }

        #[test]
        fn test_display() {
            let group = AccountGroup("All".to_string());
            assert_eq!(format!("{}", group), "All");
        }

        #[test]
        fn test_from_str() {
            let group = AccountGroup::from("All");
            assert_eq!(group.0, "All");
        }

        #[test]
        fn test_from_string() {
            let owned_string = "All".to_string();
            let group = AccountGroup::from(owned_string.clone());
            assert_eq!(group.0, "All");
            // Verify we're not unnecessarily cloning
            assert_eq!(group.0, owned_string);
        }

        #[test]
        fn test_empty_group() {
            let group = AccountGroup::from("");
            assert_eq!(group.as_str(), "");
        }

        #[test]
        fn test_special_group_names() {
            let group = AccountGroup::from("Group-1_Test.2024");
            assert_eq!(group.as_str(), "Group-1_Test.2024");
        }

        #[test]
        fn test_equality() {
            let group1 = AccountGroup::from("All");
            let group2 = AccountGroup::from("All");
            let group3 = AccountGroup::from("Group1");
            
            assert_eq!(group1, group2);
            assert_ne!(group1, group3);
        }
    }

    #[test]
    fn test_types_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        
        assert_send_sync::<AccountId>();
        assert_send_sync::<ModelCode>();
        assert_send_sync::<ContractId>();
        assert_send_sync::<AccountGroup>();
    }

    #[test]
    fn test_types_are_clone() {
        let account_id = AccountId("U123".to_string());
        let _ = account_id.clone();

        let model_code = ModelCode("MODEL".to_string());
        let _ = model_code.clone();

        let contract_id = ContractId(123);
        let _ = contract_id.clone();

        let account_group = AccountGroup("All".to_string());
        let _ = account_group.clone();
    }

    #[test]
    fn test_types_are_debug() {
        let account_id = AccountId("U123".to_string());
        let debug_str = format!("{:?}", account_id);
        assert!(debug_str.contains("AccountId"));
        assert!(debug_str.contains("U123"));

        let model_code = ModelCode("MODEL".to_string());
        let debug_str = format!("{:?}", model_code);
        assert!(debug_str.contains("ModelCode"));
        assert!(debug_str.contains("MODEL"));

        let contract_id = ContractId(123);
        let debug_str = format!("{:?}", contract_id);
        assert!(debug_str.contains("ContractId"));
        assert!(debug_str.contains("123"));

        let account_group = AccountGroup("All".to_string());
        let debug_str = format!("{:?}", account_group);
        assert!(debug_str.contains("AccountGroup"));
        assert!(debug_str.contains("All"));
    }

    #[test]
    fn test_account_id_string_operations() {
        let id = AccountId::from("U123456");
        
        // Test various string operations via Deref
        assert_eq!(id.to_uppercase(), "U123456");
        assert_eq!(id.chars().count(), 7);
        assert!(id.contains("123"));
        assert_eq!(id.replace("U", "D"), "D123456");
    }

    #[test]
    fn test_model_code_string_operations() {
        let code = ModelCode::from("model_test");
        
        // Test various string operations via Deref
        assert_eq!(code.to_uppercase(), "MODEL_TEST");
        assert!(code.starts_with("model"));
        assert!(code.ends_with("test"));
        assert_eq!(code.split('_').count(), 2);
    }
}