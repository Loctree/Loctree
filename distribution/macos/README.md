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

Apple references:

- Notarizing macOS software before distribution
- Signing Mac Software with Developer ID
