// Policy validation logic for Cedar policies

use crate::errors::{AppError, Result};
use cedar_policy::{Policy, PolicySet, Schema, Validator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Validation result for a single policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl PolicyValidationResult {
    /// Create a valid result with no errors
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the result
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add multiple warnings to the result
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }
}

/// Validation result for multiple policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchValidationResult {
    pub results: HashMap<String, PolicyValidationResult>,
    pub overall_valid: bool,
}

impl BatchValidationResult {
    /// Create a new batch validation result
    pub fn new(results: HashMap<String, PolicyValidationResult>) -> Self {
        let overall_valid = results.values().all(|r| r.is_valid);
        Self {
            results,
            overall_valid,
        }
    }

    /// Get the total number of errors across all policies
    pub fn total_errors(&self) -> usize {
        self.results.values().map(|r| r.errors.len()).sum()
    }

    /// Get the total number of warnings across all policies
    pub fn total_warnings(&self) -> usize {
        self.results.values().map(|r| r.warnings.len()).sum()
    }
}

/// Policy validator for Cedar policies
pub struct PolicyValidator {
    schema: Option<Schema>,
}

impl PolicyValidator {
    /// Create a new policy validator without schema validation
    pub fn new() -> Self {
        Self { schema: None }
    }

    /// Create a new policy validator with schema validation
    pub fn with_schema(schema: Schema) -> Self {
        Self {
            schema: Some(schema),
        }
    }

    /// Validate a single Cedar policy string
    pub fn validate_policy_string(&self, policy_str: &str) -> Result<PolicyValidationResult> {
        debug!("Validating policy string");

        // Parse the policy
        let policy = match Policy::parse(None, policy_str) {
            Ok(p) => p,
            Err(e) => {
                let errors = vec![format!("Failed to parse policy: {}", e)];
                return Ok(PolicyValidationResult::invalid(errors));
            }
        };

        // Validate syntax and semantics
        self.validate_policy(&policy)
    }

    /// Validate a parsed Cedar policy
    pub fn validate_policy(&self, policy: &Policy) -> Result<PolicyValidationResult> {
        debug!("Validating parsed policy");

        // If we have a schema, perform schema validation
        if let Some(schema) = &self.schema {
            let policy_set = PolicySet::from_policies([policy.clone()])
                .map_err(|e| AppError::ValidationError(format!("Failed to create policy set: {}", e)))?;

            let validator = Validator::new(schema.clone());
            let validation_result = validator.validate(&policy_set, cedar_policy::ValidationMode::default());

            if validation_result.validation_passed() {
                let warnings: Vec<String> = validation_result
                    .validation_warnings()
                    .map(|w| w.to_string())
                    .collect();

                let mut result = PolicyValidationResult::valid();
                if !warnings.is_empty() {
                    result = result.with_warnings(warnings);
                }
                Ok(result)
            } else {
                let errors: Vec<String> = validation_result
                    .validation_errors()
                    .map(|e| e.to_string())
                    .collect();

                Ok(PolicyValidationResult::invalid(errors))
            }
        } else {
            // Without schema, we can only validate basic syntax (which is already done by parsing)
            warn!("Validating policy without schema - only syntax validation performed");
            Ok(PolicyValidationResult::valid())
        }
    }

    /// Validate multiple policies
    pub fn validate_policies(
        &self,
        policies: &[(String, &str)],
    ) -> Result<BatchValidationResult> {
        debug!("Validating {} policies", policies.len());

        let mut results = HashMap::new();

        for (policy_id, policy_str) in policies {
            let result = self.validate_policy_string(policy_str)?;
            results.insert(policy_id.clone(), result);
        }

        Ok(BatchValidationResult::new(results))
    }

    /// Validate that a policy set has no conflicts
    pub fn validate_policy_set(&self, policy_set: &PolicySet) -> Result<PolicyValidationResult> {
        debug!("Validating policy set for conflicts");

        if let Some(schema) = &self.schema {
            let validator = Validator::new(schema.clone());
            let validation_result = validator.validate(policy_set, cedar_policy::ValidationMode::default());

            if validation_result.validation_passed() {
                let warnings: Vec<String> = validation_result
                    .validation_warnings()
                    .map(|w| w.to_string())
                    .collect();

                let mut result = PolicyValidationResult::valid();
                if !warnings.is_empty() {
                    result = result.with_warnings(warnings);
                }
                Ok(result)
            } else {
                let errors: Vec<String> = validation_result
                    .validation_errors()
                    .map(|e| e.to_string())
                    .collect();

                Ok(PolicyValidationResult::invalid(errors))
            }
        } else {
            warn!("Validating policy set without schema");
            Ok(PolicyValidationResult::valid())
        }
    }

    /// Validate policy effect (allow/deny)
    pub fn validate_effect(effect: &str) -> Result<()> {
        match effect.to_lowercase().as_str() {
            "allow" | "deny" => Ok(()),
            _ => Err(AppError::ValidationError(format!(
                "Invalid policy effect: {}. Must be 'allow' or 'deny'",
                effect
            ))),
        }
    }

    /// Validate policy status
    pub fn validate_status(status: &str) -> Result<()> {
        match status.to_lowercase().as_str() {
            "active" | "inactive" | "deleted" => Ok(()),
            _ => Err(AppError::ValidationError(format!(
                "Invalid policy status: {}. Must be 'active', 'inactive', or 'deleted'",
                status
            ))),
        }
    }

    /// Validate policy name format
    pub fn validate_policy_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(AppError::ValidationError(
                "Policy name cannot be empty".to_string(),
            ));
        }

        if name.len() > 255 {
            return Err(AppError::ValidationError(format!(
                "Policy name too long: {} characters (max 255)",
                name.len()
            )));
        }

        // Check for valid characters (alphanumeric, underscore, hyphen, space)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ')
        {
            return Err(AppError::ValidationError(
                "Policy name contains invalid characters. Only alphanumeric, underscore, hyphen, and space allowed".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate policy priority
    pub fn validate_priority(priority: i32) -> Result<()> {
        if priority < 0 {
            return Err(AppError::ValidationError(
                "Policy priority cannot be negative".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for PolicyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a basic Cedar schema for Agent IAM
pub fn create_agent_iam_schema() -> Result<Schema> {
    let schema_json = r#"{
        "AgentIAM": {
            "entityTypes": {
                "User": {
                    "memberOfTypes": ["Role"]
                },
                "Service": {
                    "memberOfTypes": ["Role"]
                },
                "Agent": {
                    "memberOfTypes": ["Role"]
                },
                "Role": {
                    "memberOfTypes": ["Role"]
                },
                "Resource": {}
            },
            "actions": {
                "read": {
                    "appliesTo": {
                        "principalTypes": ["User", "Service", "Agent"],
                        "resourceTypes": ["Resource"]
                    }
                },
                "write": {
                    "appliesTo": {
                        "principalTypes": ["User", "Service", "Agent"],
                        "resourceTypes": ["Resource"]
                    }
                },
                "delete": {
                    "appliesTo": {
                        "principalTypes": ["User", "Service", "Agent"],
                        "resourceTypes": ["Resource"]
                    }
                },
                "execute": {
                    "appliesTo": {
                        "principalTypes": ["User", "Service", "Agent"],
                        "resourceTypes": ["Resource"]
                    }
                },
                "admin": {
                    "appliesTo": {
                        "principalTypes": ["User", "Service"],
                        "resourceTypes": ["Resource"]
                    }
                }
            }
        }
    }"#;

    Schema::from_str(schema_json)
        .map_err(|e| AppError::ValidationError(format!("Failed to create schema: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_effect() {
        assert!(PolicyValidator::validate_effect("allow").is_ok());
        assert!(PolicyValidator::validate_effect("deny").is_ok());
        assert!(PolicyValidator::validate_effect("ALLOW").is_ok());
        assert!(PolicyValidator::validate_effect("DENY").is_ok());
        assert!(PolicyValidator::validate_effect("invalid").is_err());
        assert!(PolicyValidator::validate_effect("").is_err());
    }

    #[test]
    fn test_validate_status() {
        assert!(PolicyValidator::validate_status("active").is_ok());
        assert!(PolicyValidator::validate_status("inactive").is_ok());
        assert!(PolicyValidator::validate_status("deleted").is_ok());
        assert!(PolicyValidator::validate_status("ACTIVE").is_ok());
        assert!(PolicyValidator::validate_status("invalid").is_err());
        assert!(PolicyValidator::validate_status("").is_err());
    }

    #[test]
    fn test_validate_policy_name() {
        assert!(PolicyValidator::validate_policy_name("valid_name").is_ok());
        assert!(PolicyValidator::validate_policy_name("valid-name").is_ok());
        assert!(PolicyValidator::validate_policy_name("valid name").is_ok());
        assert!(PolicyValidator::validate_policy_name("Valid123").is_ok());
        assert!(PolicyValidator::validate_policy_name("").is_err());
        assert!(PolicyValidator::validate_policy_name("invalid@name").is_err());
        assert!(PolicyValidator::validate_policy_name(&"a".repeat(256)).is_err());
    }

    #[test]
    fn test_validate_priority() {
        assert!(PolicyValidator::validate_priority(0).is_ok());
        assert!(PolicyValidator::validate_priority(100).is_ok());
        assert!(PolicyValidator::validate_priority(i32::MAX).is_ok());
        assert!(PolicyValidator::validate_priority(-1).is_err());
    }

    #[test]
    fn test_validate_simple_policy() {
        let validator = PolicyValidator::new();
        let policy_str = r#"permit(principal, action, resource);"#;

        let result = validator.validate_policy_string(policy_str).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_syntax() {
        let validator = PolicyValidator::new();
        let policy_str = r#"this is not a valid policy"#;

        let result = validator.validate_policy_string(policy_str).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_multiple_policies() {
        let validator = PolicyValidator::new();
        let policies = vec![
            ("policy1".to_string(), r#"permit(principal, action, resource);"#),
            ("policy2".to_string(), r#"forbid(principal, action, resource);"#),
            ("policy3".to_string(), r#"invalid syntax"#),
        ];

        let batch_result = validator.validate_policies(&policies).unwrap();
        assert!(!batch_result.overall_valid);
        assert_eq!(batch_result.results.len(), 3);

        assert!(batch_result.results.get("policy1").unwrap().is_valid);
        assert!(batch_result.results.get("policy2").unwrap().is_valid);
        assert!(!batch_result.results.get("policy3").unwrap().is_valid);
    }

    #[test]
    fn test_create_agent_iam_schema() {
        let result = create_agent_iam_schema();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_result() {
        let valid = PolicyValidationResult::valid();
        assert!(valid.is_valid);
        assert!(valid.errors.is_empty());
        assert!(valid.warnings.is_empty());

        let invalid = PolicyValidationResult::invalid(vec!["error1".to_string()]);
        assert!(!invalid.is_valid);
        assert_eq!(invalid.errors.len(), 1);

        let with_warning = valid.with_warning("warning1".to_string());
        assert!(with_warning.is_valid);
        assert_eq!(with_warning.warnings.len(), 1);
    }

    #[test]
    fn test_batch_validation_result() {
        let mut results = HashMap::new();
        results.insert("valid".to_string(), PolicyValidationResult::valid());
        results.insert(
            "invalid".to_string(),
            PolicyValidationResult::invalid(vec!["error".to_string()]),
        );

        let batch = BatchValidationResult::new(results);
        assert!(!batch.overall_valid);
        assert_eq!(batch.total_errors(), 1);
        assert_eq!(batch.total_warnings(), 0);
    }
}
