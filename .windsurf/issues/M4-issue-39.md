# Issue #39: Developer Documentation

## Context
Create comprehensive developer documentation including README, architecture overview, API documentation, SDK guides, and getting started guides.

## Acceptance Criteria

- [ ] Main README.md:
  - Project overview and features
  - Architecture diagram
  - Quick start guide
  - Links to detailed documentation
  - Contributing guidelines
  - License information
- [ ] docs/ directory with:
  - architecture.md — System architecture overview
  - getting-started.md — Step-by-step integration guide
  - api-reference.md — REST API documentation
  - websocket-reference.md — WebSocket API documentation
  - sdk-guide.md — SDK usage guide (Rust and TypeScript)
  - deployment.md — Self-hosting guide (already exists, update if needed)
  - faq.md — Frequently asked questions
- [ ] API documentation:
  - OpenAPI/Swagger spec (openapi.yaml)
  - Interactive API explorer reference
- [ ] Code documentation:
  - Rust: cargo doc generates complete docs
  - TypeScript: TSDoc comments
- [ ] Diagrams:
  - System architecture
  - Data flow
  - Order lifecycle
- [ ] Changelog (CHANGELOG.md)
- [ ] Security policy (SECURITY.md)

## Technical Approach

### Files to Create/Update

| File | Description |
|------|-------------|
| `README.md` | Main project README |
| `docs/architecture.md` | System architecture with diagrams |
| `docs/getting-started.md` | Integration guide |
| `docs/api-reference.md` | REST API documentation |
| `docs/websocket-reference.md` | WebSocket API documentation |
| `docs/sdk-guide.md` | SDK usage guide |
| `docs/faq.md` | FAQ |
| `openapi.yaml` | OpenAPI 3.0 specification |
| `CHANGELOG.md` | Version history |
| `SECURITY.md` | Security policy |
| `CONTRIBUTING.md` | Contributing guidelines |

### Documentation Standards

- Use Markdown for all documentation
- Include code examples in multiple languages
- Use ASCII diagrams for architecture
- Keep docs in sync with code
- Reference internal docs where appropriate

## Testing

- [ ] All links work
- [ ] Code examples are valid
- [ ] OpenAPI spec validates
- [ ] cargo doc generates without warnings
