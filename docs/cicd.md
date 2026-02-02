# CI/CD Guide

This guide explains the CI/CD pipelines for Matchbook, including workflows, secrets configuration, and branch protection rules.

## Workflows Overview

| Workflow | Trigger | Description |
|----------|---------|-------------|
| `ci.yml` | Push/PR to main | Format, lint, test, build |
| `release.yml` | Version tags (v*) | Build images, publish packages, create release |
| `deploy.yml` | Manual | Deploy to Kubernetes environments |
| `security.yml` | Push/PR + weekly | Security scanning and audits |

## CI Workflow

The main CI workflow runs on every push and pull request to `main`.

### Jobs

1. **Format** - `cargo fmt --all --check`
2. **Clippy** - `cargo clippy --all-targets --all-features --workspace -- -D warnings`
3. **Test** - `cargo test --all-features --workspace`
4. **Build** - `cargo build --all-features --workspace`
5. **Anchor Build** - Build Solana program with Anchor
6. **TypeScript** - Lint and test TypeScript SDK
7. **Docker** - Build Docker images (no push)
8. **Coverage** - Generate and upload test coverage

### Caching

The workflow uses GitHub Actions cache for:
- Cargo registry (`~/.cargo/registry`, `~/.cargo/git`)
- Build artifacts (`target/`)
- npm packages (`~/.npm`)

## Release Workflow

Triggered when a version tag is pushed (e.g., `v1.0.0`).

### Steps

1. **Validate** - Check version format
2. **Docker** - Build and push multi-arch images to GHCR
3. **Publish Rust** - Publish SDK to crates.io (dry-run by default)
4. **Publish npm** - Publish TypeScript SDK to npm (dry-run by default)
5. **GitHub Release** - Create release with changelog

### Creating a Release

```bash
# Tag the release
git tag v1.0.0
git push origin v1.0.0
```

## Deploy Workflow

Manual workflow for deploying to Kubernetes environments.

### Usage

1. Go to Actions → Deploy
2. Click "Run workflow"
3. Select environment (dev/staging/prod)
4. Enter version/tag to deploy
5. Click "Run workflow"

### Environments

| Environment | Namespace | API URL |
|-------------|-----------|---------|
| dev | matchbook-dev | api-dev.matchbook.example |
| staging | matchbook-staging | api-staging.matchbook.example |
| prod | matchbook | api.matchbook.example |

### Rollback

If deployment fails, the workflow automatically rolls back to the previous version.

Manual rollback:
```bash
kubectl rollout undo deployment/api -n matchbook
```

## Security Workflow

Runs on push/PR and weekly schedule.

### Scans

- **cargo-audit** - Check Rust dependencies for vulnerabilities
- **Dependency Review** - Review dependency changes in PRs
- **Secrets Scan** - Detect leaked secrets with TruffleHog
- **CodeQL** - Static analysis for TypeScript
- **npm audit** - Check npm dependencies
- **Container Scan** - Trivy scan for Docker images

## Secrets Configuration

Configure these secrets in repository settings (Settings → Secrets and variables → Actions):

### Required Secrets

| Secret | Description | Used By |
|--------|-------------|---------|
| `CRATES_IO_TOKEN` | crates.io API token | release.yml |
| `NPM_TOKEN` | npm publish token | release.yml |
| `KUBECONFIG` | Kubernetes config (base64) | deploy.yml |

### Optional Secrets

| Secret | Description | Used By |
|--------|-------------|---------|
| `CODECOV_TOKEN` | Codecov upload token | ci.yml |
| `SOLANA_KEYPAIR` | Devnet deploy keypair | Future devnet deploy |

### Creating Secrets

#### CRATES_IO_TOKEN

1. Go to https://crates.io/settings/tokens
2. Create new token with "publish-update" scope
3. Add to repository secrets

#### NPM_TOKEN

1. Go to https://www.npmjs.com/settings/~/tokens
2. Create new "Automation" token
3. Add to repository secrets

#### KUBECONFIG

```bash
# Encode kubeconfig
cat ~/.kube/config | base64 -w 0

# Add the output as KUBECONFIG secret
```

## Branch Protection Rules

Configure these rules for the `main` branch (Settings → Branches → Add rule):

### Recommended Settings

- [x] **Require a pull request before merging**
  - [x] Require approvals: 1
  - [x] Dismiss stale pull request approvals when new commits are pushed
  - [x] Require review from Code Owners

- [x] **Require status checks to pass before merging**
  - [x] Require branches to be up to date before merging
  - Required checks:
    - `Format`
    - `Clippy`
    - `Test`
    - `Build`
    - `TypeScript`

- [x] **Require conversation resolution before merging**

- [x] **Do not allow bypassing the above settings**

## Dependabot

Dependabot is configured to check for updates weekly:

- **Rust** - Cargo dependencies
- **npm** - TypeScript SDK dependencies
- **GitHub Actions** - Workflow action versions
- **Docker** - Base image versions

### Grouping

Minor and patch updates are grouped to reduce PR noise:
- `rust-minor` - All Rust minor/patch updates
- `npm-minor` - All npm minor/patch updates

## Troubleshooting

### CI Failures

#### Format Check Failed
```bash
cargo fmt --all
git add -A && git commit -m "chore: format code"
```

#### Clippy Warnings
```bash
cargo clippy --fix --all-targets --all-features --allow-dirty
```

#### Test Failures
```bash
cargo test --all-features --workspace -- --nocapture
```

### Docker Build Failures

Check the Dockerfile syntax and ensure all required files are present:
```bash
docker build -f indexer/Dockerfile .
```

### Deploy Failures

Check pod status:
```bash
kubectl get pods -n matchbook
kubectl describe pod <pod-name> -n matchbook
kubectl logs <pod-name> -n matchbook
```

## Related Documentation

- [Docker Guide](./docker.md)
- [Kubernetes Guide](../k8s/README.md)
- [Deployment Guide](../.internalDoc/07-deployment.md)
