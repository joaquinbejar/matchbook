# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

**DO NOT** open a public GitHub issue for security vulnerabilities.

Instead, please email us at: **security@matchbook.taunais.com**

Include the following information:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Any suggested fixes (optional)

### What to Expect

1. **Acknowledgment**: We will acknowledge receipt within 48 hours
2. **Assessment**: We will assess the vulnerability and determine severity
3. **Updates**: We will keep you informed of our progress
4. **Resolution**: We aim to resolve critical issues within 7 days
5. **Disclosure**: We will coordinate disclosure timing with you

### Severity Levels

| Severity | Description | Response Time |
|----------|-------------|---------------|
| Critical | Funds at risk, remote code execution | 24 hours |
| High | Significant impact, data exposure | 72 hours |
| Medium | Limited impact, requires specific conditions | 1 week |
| Low | Minimal impact, informational | 2 weeks |

## Security Measures

### On-Chain Program

- **Audited**: Program code is audited by [Auditor Name]
- **Non-custodial**: Users maintain control of their funds
- **Checked arithmetic**: All math operations use checked arithmetic
- **Access control**: Operations require appropriate signatures

### Off-Chain Services

- **Authentication**: API keys and signed requests for sensitive operations
- **Rate limiting**: Protection against abuse
- **Input validation**: All inputs are validated and sanitized
- **TLS**: All communications are encrypted

### Infrastructure

- **Kubernetes**: Deployed with security best practices
- **Secrets management**: Sensitive data stored in Kubernetes secrets
- **Network policies**: Restricted inter-service communication
- **Monitoring**: Continuous security monitoring and alerting

## Bug Bounty Program

We offer rewards for responsibly disclosed vulnerabilities:

| Severity | Reward |
|----------|--------|
| Critical | Up to $50,000 |
| High | Up to $10,000 |
| Medium | Up to $2,000 |
| Low | Up to $500 |

### Scope

**In Scope:**
- On-chain program (`program/`)
- API server (`api/`)
- SDK vulnerabilities that affect users
- Authentication/authorization bypasses

**Out of Scope:**
- Social engineering attacks
- Denial of service attacks
- Issues in third-party dependencies (report to upstream)
- Issues requiring physical access

### Rules

- Do not access or modify other users' data
- Do not perform actions that could harm users
- Do not publicly disclose before we've had time to fix
- Provide sufficient detail to reproduce the issue

## Security Best Practices for Users

### Wallet Security

- Use hardware wallets for significant funds
- Never share your private keys or seed phrase
- Verify transaction details before signing

### API Security

- Keep API keys confidential
- Use environment variables, not hardcoded keys
- Rotate keys periodically
- Use the minimum required permissions

### Integration Security

- Validate all data from the API
- Implement proper error handling
- Use HTTPS for all communications
- Keep SDKs updated to latest versions

## Contact

- Security issues: security@matchbook.taunais.com
- General inquiries: support@matchbook.taunais.com
- PGP key: Available at https://matchbook.taunais.com/.well-known/security.txt
