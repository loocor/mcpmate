# MCPMate Tests

This directory contains the test code for the MCPMate project, using a layered test strategy including unit tests and integration tests.

## Test directory structure

```
/tests
├── api/                          # API tests
│   ├── health.rs                 # Health endpoint tests
│   ├── servers.rs                # Server management API tests
│   ├── tools.rs                  # Tool management API tests
│   ├── suits.rs                  # Config suit API tests
│   ├── system.rs                 # System API tests
│   └── mod.rs                    # Module exports
├── common/                       # Shared test utilities and helpers
│   ├── environment.rs            # Test environment setup
│   ├── fixtures.rs               # Test fixtures and data helpers
│   ├── mocks.rs                  # Mock implementations
│   └── mod.rs                    # Module exports
├── integration/                  # Integration tests
│   ├── proxy.rs                  # Proxy integration tests
│   ├── bridge.rs                 # Bridge integration tests
│   ├── end_to_end.rs             # End-to-end workflow tests
│   └── mod.rs                    # Module exports
├── modules/                      # Modules functionality tests
│   ├── config/                   # Configuration tests
│   │   ├── loading.rs            # Config loading tests
│   │   ├── validation.rs         # Config validation tests
│   │   └── mod.rs                # Module exports
│   ├── server/                   # Server tests
│   │   ├── connection.rs         # Server connection tests
│   │   ├── lifecycle.rs          # Server lifecycle tests
│   │   └── mod.rs                # Module exports
│   ├── tool/                     # Tool tests
│   │   ├── calling.rs            # Tool calling tests
│   │   ├── mapping.rs            # Tool mapping tests
│   │   ├── routing.rs            # Tool routing tests
│   │   └── mod.rs                # Module exports
│   └── mod.rs                    # Module exports
└── lib.rs                        # Test library entry point
```

## Test strategy

### Unit tests
- Test independent functions and methods
- Use mock objects to isolate external dependencies
- Fast execution, high coverage

### Integration tests
- Test interactions between modules
- Test end-to-end functionality
- Use real dependencies or mock services

## Test tools

- **Test framework**: Rust's built-in `#[test]`
- **Async tests**: `tokio::test`
- **Mocking**: `mockall` or `mockito`
- **HTTP tests**: `reqwest` and `wiremock`
- **Assertions**: `assert_matches`, `assert_cmd`

## Running tests

Run all tests:

```bash
cargo test
```

Run specific test modules:

```bash
cargo test --test prefix_test
```

Generate test coverage report (requires `grcov` and `llvm-tools`):

```bash
cargo install grcov
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace --tests
```

## Test coverage targets

- Core functionality: 90%+
- Tool functions: 85%+
- Integration tests: 80%+

## Code style

- Test function names should clearly describe the test scenario and expected result
- Use `#[test]` attribute to mark test functions
- Use `#[tokio::test]` attribute to mark async tests
- Use `#[ignore]` attribute to mark tests that should be skipped
- Use `#[should_panic]` attribute to mark tests that should panic

## Continuous integration

The project is configured with GitHub Actions workflows that automatically run tests on every push and pull request.

## Test data

- Test data should be placed in the `tests/fixtures/` directory
- Use `include_str!` or `include_bytes!` macros to load test data
- Avoid hardcoding large amounts of data in tests