# Windows

Windows distribution is release-asset first:

- zipped `loctree.exe` and `loct.exe`
- npm wrapper only for targets we actually build in CI

If we add winget later, it should join this tree rather than becoming another
root-level ritual.
