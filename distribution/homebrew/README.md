# Homebrew

Canonical shape:

- formula source of truth lives at `distribution/homebrew/Formula/loctree.rb`
- helper tooling lives next to it
- published tap lives in `Loctree/homebrew-loctree`

Decision:

- use a custom tap now
- treat `homebrew-core` as a later nice-to-have, not the primary path

Why:

- the tap already exists
- we control merge velocity
- we avoid pretending `brew install loctree` is live before it actually is

User-facing install path once live:

```bash
brew tap Loctree/loctree
brew install loctree
```
