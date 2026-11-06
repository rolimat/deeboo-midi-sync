# Testing in Rust

> Generic reference adapted from Jeffallan/claude-skills (MIT), trimmed for this project.
> The project overrides in [../SKILL.md](../SKILL.md) win over anything here.

> Where tests live here and the simulated-time helpers: the `writing-tests` skill. No tokio, proptest, mockall, criterion, insta or fuzzing in this project.

## Unit Tests

```rust
// Tests in same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_subtraction() {
        assert!(10 - 5 == 5);
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_panic() {
        divide(10, 0);
    }

    #[test]
    fn test_result() -> Result<(), String> {
        let result = divide(10, 2)?;
        assert_eq!(result, 5);
        Ok(())
    }

    #[test]
    #[ignore]
    fn expensive_test() {
        // Run with: cargo test -- --ignored
    }
}

// Assertions
fn assert_examples() {
    assert!(true);
    assert_eq!(2 + 2, 4);
    assert_ne!(2 + 2, 5);

    // Custom messages
    assert!(value > 0, "Value must be positive, got {}", value);
    assert_eq!(result, expected, "Calculation failed");
}
```

## Doctests

```rust
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use mylib::add;
///
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
///
/// ```should_panic
/// use mylib::divide;
///
/// divide(10, 0);  // This will panic
/// ```
///
/// ```ignore
/// // This code won't compile but won't fail the test
/// let x = undefined_function();
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Integration Tests

```rust
// tests/integration_test.rs
use mylib;

#[test]
fn test_full_workflow() {
    let config = mylib::Config::new("test.conf");
    let result = mylib::process(&config);
    assert!(result.is_ok());
}

// tests/common/mod.rs - shared test utilities
pub fn setup() -> TestContext {
    TestContext {
        db: create_test_db(),
    }
}

// tests/another_test.rs
mod common;

#[test]
fn test_with_common() {
    let ctx = common::setup();
    // Use ctx...
}
```

## Test Organization

```rust
// Nested test modules
#[cfg(test)]
mod tests {
    use super::*;

    mod addition {
        use super::*;

        #[test]
        fn positive_numbers() {
            assert_eq!(add(2, 3), 5);
        }

        #[test]
        fn negative_numbers() {
            assert_eq!(add(-2, -3), -5);
        }
    }

    mod subtraction {
        use super::*;

        #[test]
        fn test_subtract() {
            assert_eq!(subtract(10, 5), 5);
        }
    }
}
```

## Test Fixtures and Setup

```rust
struct TestContext {
    temp_dir: std::path::PathBuf,
    db: Database,
}

impl TestContext {
    fn setup() -> Self {
        let temp_dir = std::env::temp_dir().join("test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        Self {
            temp_dir,
            db: Database::connect_test(),
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Cleanup
        std::fs::remove_dir_all(&self.temp_dir).ok();
        self.db.disconnect();
    }
}

#[test]
fn test_with_fixture() {
    let ctx = TestContext::setup();
    // Test uses ctx...
    // Automatic cleanup via Drop
}
```

## Best Practices

- TDD: write the test first and see it fail before the code exists.
- Unit tests in `#[cfg(test)]` modules; behaviour tests in `tests/` (clock-core uses simulated time).
- Descriptive `snake_case_behaviour` names; one behaviour per test.
- Test edge cases: empty input, zero/equal timestamps, boundaries (e.g. exactly 500 ms).
- Tests may `unwrap()`; production code may not.
- Doctests only where an example clarifies a public API.
- No real MIDI or wall clock in automated tests; `just precommit` runs everything in Docker.
