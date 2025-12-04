# Contributing to Loctree

## Git Hooks Setup

After cloning, run this once to enable pre-push quality gate:

```bash
./scripts/install-hooks.sh
```

This creates a symlink from `.git/hooks/pre-push` to our versioned `hooks/pre-push` script, ensuring all contributors run the same checks (clippy, tests, formatting) before pushing.

## Development Workflow

```bash
cargo build              # Build
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
./hooks/pre-push         # Manual quality gate test
```

## Pull Requests

- Run `./hooks/pre-push` before submitting
- Keep commits atomic and well-described
- Update docs if adding new features

---

*Developed with care by The Loctree Team (c)2025*
