# M1 Issue #1: Project Scaffolding

## Overview

Set up the foundational project structure for the Matchbook on-chain program, including Cargo workspace configuration, Anchor framework setup, and basic CI pipeline.

## Acceptance Criteria

- [ ] Cargo workspace configured with `program/` member
- [ ] Anchor.toml configured for devnet and mainnet
- [ ] Basic `lib.rs` with program ID placeholder
- [ ] `Cargo.toml` with required dependencies (anchor-lang, anchor-spl, solana-program)
- [ ] GitHub Actions workflow for `cargo build` and `cargo test`
- [ ] `rustfmt.toml` and `.clippy.toml` for code style
- [ ] Basic README with build instructions

## Technical Approach

### 1. Project Structure

```
matchbook/
├── Cargo.toml              # Workspace root (update)
├── Anchor.toml             # Anchor configuration (new)
├── program/                # On-chain program (new)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── .github/
│   └── workflows/
│       └── ci.yml          # CI workflow (new)
├── rustfmt.toml            # Formatting config (new)
└── .clippy.toml            # Clippy config (new)
```

### 2. Dependencies

Based on `.internalDoc/03-onchain-design.md`:
- `anchor-lang = "0.30"` - Anchor framework
- `anchor-spl = "0.30"` - SPL token integration
- `solana-program = "2.0"` - Solana runtime

### 3. Linting Configuration

Based on `.internalDoc/09-rust-guidelines.md`:
- Deny `unwrap_used`, `expect_used`, `panic` in clippy
- Warn on `missing_docs`, `indexing_slicing`
- Configure cognitive complexity threshold

### 4. CI Pipeline

GitHub Actions workflow:
- Trigger on push/PR to main
- Run `cargo fmt --check`
- Run `cargo clippy -- -D warnings`
- Run `cargo test`
- Build with `anchor build`

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `program` to workspace members, add workspace dependencies |
| `program/Cargo.toml` | Create | Program crate configuration |
| `program/src/lib.rs` | Create | Program entry point with placeholder |
| `Anchor.toml` | Create | Anchor configuration |
| `.github/workflows/ci.yml` | Create | CI workflow |
| `rustfmt.toml` | Create | Formatting configuration |
| `.clippy.toml` | Create | Clippy configuration |

## Tests Needed

- Basic compilation test (implicit via CI)
- Program builds successfully with `anchor build`

## Notes

- Program ID will be generated on first `anchor build`
- Edition 2024 requires Rust 1.85+ (current stable is 1.84), will use 2021 for compatibility
- Anchor 0.30 is the latest stable version
