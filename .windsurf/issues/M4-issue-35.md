# Issue #35: CI/CD pipelines with GitHub Actions

## Acceptance Criteria

- [ ] CI workflow (`.github/workflows/ci.yml`):
  - Trigger on push and PR to main
  - Rust: `cargo fmt --check`, `cargo clippy`, `cargo test`
  - TypeScript: `npm run lint`, `npm run test`
  - Build all Docker images
  - Run integration tests
  - Upload test coverage reports
- [ ] On-chain program workflow:
  - Build with `anchor build`
  - Run `anchor test`
  - Verify program size within limits
  - Deploy to devnet on main branch merge
- [ ] Release workflow:
  - Trigger on version tags
  - Build and push Docker images to registry
  - Publish Rust crate to crates.io
  - Publish TypeScript package to npm
  - Create GitHub release with changelog
- [ ] Deploy workflow:
  - Manual trigger with environment selection
  - Deploy to Kubernetes cluster
  - Run smoke tests post-deployment
  - Rollback on failure
- [ ] Security scanning (Dependabot, cargo-audit)
- [ ] Branch protection rules documentation
- [ ] Secrets management documentation

## Technical Approach

### Workflow Structure

```
.github/
├── workflows/
│   ├── ci.yml           # Main CI pipeline
│   ├── release.yml      # Release and publish
│   ├── deploy.yml       # Manual deployment
│   └── security.yml     # Security scanning
├── dependabot.yml       # Dependency updates
└── CODEOWNERS           # Code ownership
```

### CI Workflow Strategy

1. **Format check** - Fast fail on formatting issues
2. **Lint** - Clippy for Rust, ESLint for TypeScript
3. **Test** - Unit and integration tests with coverage
4. **Build** - Docker images for all services
5. **Integration** - End-to-end tests (optional)

### Caching Strategy

- Rust: `~/.cargo/registry`, `~/.cargo/git`, `target/`
- Node: `~/.npm`, `node_modules/`
- Docker: Layer caching with buildx

### Secrets Required

| Secret | Description |
|--------|-------------|
| `DOCKERHUB_USERNAME` | Docker Hub username |
| `DOCKERHUB_TOKEN` | Docker Hub access token |
| `CRATES_IO_TOKEN` | crates.io API token |
| `NPM_TOKEN` | npm publish token |
| `KUBECONFIG` | Kubernetes config (base64) |
| `SOLANA_KEYPAIR` | Devnet deploy keypair |

## Files to Create

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `.github/workflows/deploy.yml`
- `.github/workflows/security.yml`
- `.github/dependabot.yml`
- `docs/cicd.md`

## Implementation Steps

1. Create CI workflow with Rust and TypeScript checks
2. Create release workflow for versioned releases
3. Create deploy workflow for manual deployments
4. Create security workflow with cargo-audit
5. Configure Dependabot
6. Add documentation

## Testing

- [ ] CI workflow runs successfully on push
- [ ] All checks pass (fmt, clippy, test)
- [ ] Docker images build correctly
- [ ] Release workflow validates on tag
