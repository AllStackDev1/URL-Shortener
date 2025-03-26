use url::Url;
use chrono::{DateTime, Utc};

use validator::ValidationError;

/// Validates that a URL string is properly formatted and uses http/https
pub fn validate_url(url_str: &str) -> Result<(), ValidationError> {
    match Url::parse(url_str) {
        Ok(url) => {
            // Ensure URL has a scheme and host
            if url.scheme().is_empty() || url.host().is_none() {
                return Err(ValidationError::new("URL must have a scheme and host"));
            }

            // Only accept HTTP and HTTPS URLs
            if url.scheme() != "http" && url.scheme() != "https" {
                return Err(ValidationError::new("URL scheme must be http or https"));
            }

            Ok(())
        }
        Err(_) => Err(ValidationError::new("Invalid URL format")),
    }
}

/// Validates that a custom alias (if provided) meets requirements:
/// - Between 1-100 characters
/// - Only contains URL-safe characters
pub fn validate_custom_alias(alias: &str) -> Result<(), ValidationError> {
    // Check length
    if alias.is_empty() || alias.len() > 10 {
        let mut err = ValidationError::new("custom_alias_length");
        err.message = Some("Custom alias must be between 1 and 10 characters".into());
        return Err(err);
    }

    // Ensure it only contains URL-safe characters
    if !alias
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ValidationError::new(
            "Custom alias can only contain alphanumeric characters, hyphens, and underscores",
        ));
    }

    Ok(())
}


/// Validates that a date is in the future
pub fn validate_date(date_str: &DateTime<Utc>) -> Result<(), ValidationError> {
    // Ensure the date is in the future
    if date_str < &Utc::now() {
        return Err(ValidationError::new("Date must be in the future"));
    }

    Ok(())

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url() {
        // Valid URLs
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com/path?query=value").is_ok());

        // Invalid URLs
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://example.com").is_err()); // Not http/https
    }

    #[test]
    fn test_validate_custom_alias() {
        // Valid aliases
        assert!(validate_custom_alias("valid-alias").is_ok());
        assert!(validate_custom_alias("valid_alias123").is_ok());

        // Invalid aliases
        let too_long = "a".repeat(101);
        assert!(validate_custom_alias(&too_long).is_err());
        assert!(validate_custom_alias("invalid/alias").is_err());
    }

    #[test]
    fn test_validate_date() {
        // Valid dates
        let future_date = Utc::now() + chrono::Duration::days(1);
        assert!(validate_date(&future_date).is_ok());

        // Invalid dates
        let past_date = Utc::now() - chrono::Duration::days(1);
        assert!(validate_date(&past_date).is_err());
    }
}
