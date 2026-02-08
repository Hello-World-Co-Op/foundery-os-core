//! Authorization Module for FounderyOS Core
//!
//! FOS-5.6.12: User Data Backend Authorization
//! Addresses CAP-001 (CRITICAL), WS-001 (CRITICAL), CAP-003 (HIGH)
//!
//! This module provides consistent authorization patterns for all
//! data operations in the canister, following defense-in-depth principles.
//!
//! @see AC-5.6.12.1 - Capture CRUD authorization
//! @see AC-5.6.12.2 - Document CRUD authorization
//! @see AC-5.6.12.3 - Workspace CRUD authorization
//! @see AC-5.6.12.6 - Error messages without data leakage

use candid::Principal;

/// Unauthorized access error message.
/// Uses generic message to prevent information leakage (AC-5.6.12.6)
const UNAUTHORIZED_ERROR: &str = "Unauthorized: You do not own this resource";

/// Verify that the caller is the owner of a resource.
///
/// # Arguments
/// * `owner` - The Principal that owns the resource
///
/// # Returns
/// * `Ok(())` if caller matches owner
/// * `Err(String)` with unauthorized message if not
///
/// # Example
/// ```rust
/// let capture = state.get_capture(id)?;
/// require_owner(capture.owner)?;
/// // caller is authorized to access this capture
/// ```
///
/// Note: Currently lib.rs uses inline authorization checks for better control flow.
/// This function is available for future refactoring or external module use.
#[allow(dead_code)]
pub fn require_owner(owner: Principal) -> Result<(), String> {
    let caller = ic_cdk::caller();
    if caller != owner {
        return Err(UNAUTHORIZED_ERROR.to_string());
    }
    Ok(())
}

/// Check if the caller is the owner of a resource.
///
/// # Arguments
/// * `owner` - The Principal that owns the resource
///
/// # Returns
/// * `true` if caller matches owner
/// * `false` otherwise
///
/// # Example
/// ```rust
/// if is_owner(resource.owner) {
///     // show edit controls
/// }
/// ```
///
/// Note: Currently lib.rs uses inline checks. Available for future use.
#[allow(dead_code)]
pub fn is_owner(owner: Principal) -> bool {
    ic_cdk::caller() == owner
}

/// Verify that the caller is the owner by user_id (for session-based auth).
///
/// # Arguments
/// * `owner_user_id` - The user_id string that owns the resource
/// * `caller_user_id` - The authenticated user_id of the caller
///
/// # Returns
/// * `Ok(())` if user_ids match
/// * `Err(String)` with unauthorized message if not
pub fn require_owner_by_user_id(owner_user_id: &str, caller_user_id: &str) -> Result<(), String> {
    if owner_user_id != caller_user_id {
        return Err(UNAUTHORIZED_ERROR.to_string());
    }
    Ok(())
}

/// Check if the caller user_id matches the owner user_id.
///
/// # Arguments
/// * `owner_user_id` - The user_id string that owns the resource
/// * `caller_user_id` - The authenticated user_id of the caller
///
/// # Returns
/// * `true` if user_ids match
/// * `false` otherwise
pub fn is_owner_by_user_id(owner_user_id: &str, caller_user_id: &str) -> bool {
    owner_user_id == caller_user_id
}

/// Verify that the caller is not anonymous.
///
/// # Returns
/// * `Ok(Principal)` with the caller's principal if authenticated
/// * `Err(String)` if caller is anonymous
///
/// Note: lib.rs has its own require_authenticated(). This is available for other modules.
#[allow(dead_code)]
pub fn require_authenticated() -> Result<Principal, String> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        return Err("Authentication required".to_string());
    }
    Ok(caller)
}

/// Get the caller principal if authenticated, None if anonymous.
///
/// # Returns
/// * `Some(Principal)` if caller is authenticated
/// * `None` if caller is anonymous
///
/// Note: Available for modules that need optional authentication checks.
#[allow(dead_code)]
pub fn get_authenticated_caller() -> Option<Principal> {
    let caller = ic_cdk::caller();
    if caller == Principal::anonymous() {
        None
    } else {
        Some(caller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: ic_cdk::caller() is not available in unit tests.
    // Authorization logic is tested via PocketIC integration tests.
    // See: tests/pocketic_smoke.rs

    #[test]
    fn test_require_owner_by_user_id_same() {
        let owner = "user123";
        let caller = "user123";
        assert!(require_owner_by_user_id(owner, caller).is_ok());
    }

    #[test]
    fn test_require_owner_by_user_id_different() {
        let owner = "user123";
        let caller = "user456";
        let result = require_owner_by_user_id(owner, caller);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), UNAUTHORIZED_ERROR);
    }

    #[test]
    fn test_is_owner_by_user_id_same() {
        assert!(is_owner_by_user_id("user123", "user123"));
    }

    #[test]
    fn test_is_owner_by_user_id_different() {
        assert!(!is_owner_by_user_id("user123", "user456"));
    }

    #[test]
    fn test_error_message_no_leakage() {
        // Verify error message doesn't reveal whether resource exists
        // or any information about the owner
        assert!(!UNAUTHORIZED_ERROR.contains("not found"));
        assert!(!UNAUTHORIZED_ERROR.contains("owner"));
        assert!(!UNAUTHORIZED_ERROR.contains("principal"));
    }
}
