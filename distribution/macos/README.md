# macOS

This channel is for direct Loctree downloads outside Homebrew and npm.

Target shape:

- signed `loctree` and `loct` binaries
- notarized archive for direct download
- optional installer package later if we want a friendlier non-terminal path

Current direction:

- sign binaries with Developer ID Application
- notarize a zipped bundle with `notarytool`
- upload the notarized macOS asset to GitHub Releases
- run `distribution/macos/smoke-releaseability.sh` before packaging so releases fail on non-system dylib paths such as `/opt/homebrew/...`

Releaseability smoke path:

```bash
make smoke-release-macos-arm64
```

Apple references:

- Notarizing macOS software before distribution
- Signing Mac Software with Developer ID
