# Contributing to Matchbook

Thank you for your interest in contributing to Matchbook! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

### Prerequisites

- Rust 1.75+
- Solana CLI 1.18+
- Node.js 18+ (for TypeScript SDK)
- Docker (for local development)

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/matchbook.git
   cd matchbook
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/joaquinbejar/matchbook.git
   ```
4. Install dependencies:
   ```bash
   # Rust
   cargo build
   
   # TypeScript SDK
   cd ts-sdk && npm install
   ```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

### 2. Make Changes

- Follow the coding standards (see below)
- Write tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test --all-features

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all --check

# TypeScript SDK
cd ts-sdk && npm test
```

### 4. Commit Your Changes

Write clear, concise commit messages:

```bash
git commit -m "feat: add new order type support"
git commit -m "fix: resolve race condition in matching"
git commit -m "docs: update API reference"
```

Follow [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Description |
|--------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation |
| `refactor` | Code refactoring |
| `test` | Adding tests |
| `chore` | Maintenance |

### 5. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

## Coding Standards

### Rust

Follow the guidelines in `.internalDoc/09-rust-guidelines.md`:

- Use `#[must_use]` on pure functions
- Use checked arithmetic (no `.unwrap()` in production code)
- Write doc comments on all public items
- Keep functions focused and small
- Use meaningful variable names

Example:

```rust
/// Calculates the total value of an order.
///
/// # Arguments
///
/// * `price` - The price per unit in base units
/// * `quantity` - The quantity in base units
///
/// # Returns
///
/// The total value, or `None` if overflow occurs.
#[must_use]
pub fn calculate_total(price: u64, quantity: u64) -> Option<u64> {
    price.checked_mul(quantity)
}
```

### TypeScript

- Use TypeScript strict mode
- Document public APIs with TSDoc
- Use meaningful variable names
- Prefer `const` over `let`

Example:

```typescript
/**
 * Calculates the total value of an order.
 * @param price - The price per unit
 * @param quantity - The quantity
 * @returns The total value
 */
export function calculateTotal(price: bigint, quantity: bigint): bigint {
  return price * quantity;
}
```

## Pull Request Guidelines

### Before Submitting

- [ ] Tests pass locally
- [ ] Code follows style guidelines
- [ ] Documentation is updated
- [ ] Commit messages are clear
- [ ] PR description explains the changes

### PR Description Template

```markdown
## Summary

[Brief description of changes]

## Changes

- [Change 1]
- [Change 2]

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Related Issues

Closes #[issue_number]
```

### Review Process

1. Automated CI checks must pass
2. At least one maintainer review required
3. Address review feedback promptly
4. Squash commits before merge (if requested)

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_total() {
        assert_eq!(calculate_total(100, 10), Some(1000));
        assert_eq!(calculate_total(u64::MAX, 2), None);
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/integration_test.rs
use matchbook_program::*;

#[tokio::test]
async fn test_place_order_flow() {
    // Test implementation
}
```

### Property-Based Tests

For critical algorithms, use property-based testing:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_price_conversion_roundtrip(price in 1u64..u64::MAX) {
        let converted = to_human(price);
        let back = to_native(converted);
        prop_assert_eq!(price, back);
    }
}
```

## Documentation

### Code Documentation

- Document all public items
- Include examples in doc comments
- Use `# Examples` section for code samples

### User Documentation

- Update relevant docs in `docs/`
- Keep examples up to date
- Use clear, concise language

## Getting Help

- **Questions**: Open a [Discussion](https://github.com/joaquinbejar/matchbook/discussions)
- **Bugs**: Open an [Issue](https://github.com/joaquinbejar/matchbook/issues)
- **Security**: See [SECURITY.md](SECURITY.md)
- **Chat**: Join our [Discord](https://discord.gg/matchbook)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
