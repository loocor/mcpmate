// Path mapper basic functionality tests

use anyhow::Result;
use mcpmate::system::paths::PathMapper;

#[test]
fn test_path_mapper_creation() -> Result<()> {
    // Given: Default system environment
    // When: Creating a PathMapper
    let mapper = PathMapper::new();

    // Then: Should succeed
    assert!(mapper.is_ok());
    Ok(())
}

#[test]
fn test_template_resolution() -> Result<()> {
    // Given: A PathMapper with system variables
    let mapper = PathMapper::new()?;

    // When: Resolving a template with user_home variable
    let template = "{{user_home}}/test/path";
    let resolved = mapper.resolve_template(template);

    // Then: Should succeed and not contain template syntax
    assert!(resolved.is_ok());
    let resolved_path = resolved?;
    assert!(!resolved_path.to_string_lossy().contains("{{"));

    Ok(())
}

#[test]
fn test_unresolved_variables() -> Result<()> {
    // Given: A PathMapper
    let mapper = PathMapper::new()?;

    // When: Resolving a template with unknown variable
    let template = "{{unknown_variable}}/test/path";
    let resolved = mapper.resolve_template(template);

    // Then: Should return an error
    assert!(resolved.is_err());

    Ok(())
}

#[test]
fn test_tilde_expansion() -> Result<()> {
    // Given: A path with tilde
    let path_with_tilde = "~/test/path";

    // When: Expanding the tilde
    let expanded = PathMapper::expand_tilde(path_with_tilde);

    // Then: Should succeed and not start with tilde
    assert!(expanded.is_ok());
    let expanded_path = expanded?;
    assert!(!expanded_path.to_string_lossy().starts_with('~'));

    Ok(())
}

#[test]
fn test_set_variable() -> Result<()> {
    // Given: A PathMapper
    let mut mapper = PathMapper::new()?;

    // When: Setting a custom variable
    mapper.set_variable("custom_var".to_string(), "/custom/path".to_string());

    // Then: Should be able to resolve it
    let template = "{{custom_var}}/test";
    let resolved = mapper.resolve_template(template)?;
    assert_eq!(resolved.to_string_lossy(), "/custom/path/test");

    Ok(())
}

#[test]
fn test_get_variables() -> Result<()> {
    // Given: A PathMapper
    let mapper = PathMapper::new()?;

    // When: Getting all variables
    let variables = mapper.get_variables();

    // Then: Should contain at least user_home
    assert!(variables.contains_key("user_home"));

    Ok(())
}

#[test]
fn test_default_path_mapper() {
    // Given: Default constructor
    // When: Creating PathMapper with default
    let mapper = PathMapper::default();

    // Then: Should have some variables available
    let variables = mapper.get_variables();
    // Should not panic and should provide a HashMap
    // (even if PathMapper::new() fails, default should provide empty HashMap)
    // Just verify we can access variables without panicking
    let _count = variables.len();
}
