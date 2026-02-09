# Fix: Git Root Discovery from Nested Directories

**Issue**: Git context missing when running loctree from subdirectories
**Fixed in**: `v0.8.9`
**Affected versions**: `<= v0.8.8`

## Problem Description

Loctree was failing to detect git repository context when invoked from nested subdirectories, git worktrees, or certain monorepo structures.

### Symptoms

- Snapshot paths missing `branch@commit` segment (falling back to legacy path)
- `.gitignore` rules not being applied correctly
- `git_context` showing `null` values in snapshot metadata
- Different behavior depending on which directory you ran `loct` from

### Example of Affected Scenario

```bash
# From project root - works
$ cd /project
$ loct scan
[OK] Saved to ./.loctree/main@abc1234/snapshot.json

# From nested directory - fails to detect git
$ cd /project/src/deep/nested/module
$ loct scan
[OK] Saved to ./.loctree/snapshot.json  # Missing branch@commit!
```

## Root Cause

Two components used shell commands that assumed the working directory was already inside a git repository:

### 1. GitIgnoreChecker (fs_utils.rs)

```rust
// OLD: Only works if `root` is directly in a git repo
Command::new("git")
    .arg("-C")
    .arg(root)  // <- If this isn't in a git repo, fails
    .arg("rev-parse")
    .arg("--show-toplevel")
```

### 2. get_git_info (snapshot.rs)

```rust
// OLD: Commands run with root as working directory
Command::new("git")
    .args(["rev-parse", "--abbrev-ref", "HEAD"])
    .current_dir(root)  // <- Assumes root is in a git repo
```

Neither implementation searched **upward** from the given path to find the actual `.git` directory.

### When It Broke

The bug manifested when loctree was called with a `root` parameter that wasn't directly inside a git repository, or when the scan was initiated from outside the repo:

```
/project/.git/                    <- Git root
/project/src/module/              <- User runs: loct scan /project/src/module

# Old code did:
git -C /project/src/module rev-parse --show-toplevel
# This actually WORKS (git searches upward)

# But GitIgnoreChecker passed absolute paths that confused the logic,
# and get_git_info assumed current_dir was sufficient without explicit
# upward discovery. Edge cases like worktrees (.git as file) failed.
```

The real issues were:
1. **Inconsistent path handling** between different git operations
2. **No explicit upward search** - relied on git's implicit behavior which varied
3. **Worktrees not handled** - `.git` as a file broke assumptions

The issue was particularly problematic for:
- **Monorepos**: Multiple packages, running from a package subdirectory
- **Git worktrees**: Where `.git` is a file pointing to the main repo
- **Deep directory structures**: Common in large projects

## Solution

### New Utility Function

Added `find_git_root()` in `git.rs` that uses libgit2's `Repository::discover()`:

```rust
/// Find the git repository root by searching upward from the given path.
///
/// Uses libgit2's `Repository::discover()` which properly handles:
/// - Nested directories (searches upward to find .git)
/// - Git worktrees (where .git is a file, not a directory)
/// - Submodules
///
/// Returns `None` if no git repository is found.
pub fn find_git_root(path: &Path) -> Option<PathBuf> {
    Repository::discover(path)
        .ok()
        .and_then(|repo| repo.workdir().map(|p| p.to_path_buf()))
}
```

### Updated GitIgnoreChecker

```rust
impl GitIgnoreChecker {
    pub fn new(root: &Path) -> Option<Self> {
        // Use libgit2 to find git root (searches upward properly)
        let repo_root = crate::git::find_git_root(root)?;
        Some(Self { repo_root })
    }
}
```

### Updated get_git_info

```rust
fn get_git_info(root: &Path) -> (Option<String>, Option<String>, Option<String>) {
    // Find the actual git root (searches upward from root)
    let git_root = match crate::git::find_git_root(root) {
        Some(r) => r,
        None => return (None, None, None),
    };

    // Now run git commands from the discovered root
    let repo = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&git_root)  // <- Use discovered root
        // ...
}
```

## Files Changed

```
loctree_rs/src/git.rs
├── find_git_root()                              - new utility function
├── test_find_git_root_from_repo_root()          - test from root
├── test_find_git_root_from_nested_dir()         - test from deep nested
├── test_find_git_root_non_git_dir()             - test non-git returns None
├── test_find_git_root_worktree()                - test worktree support
└── test_find_git_root_nested_repo_chooses_closest() - test nested repo picks closest

loctree_rs/src/fs_utils.rs
└── GitIgnoreChecker::new()                      - now uses find_git_root()

loctree_rs/src/snapshot.rs
└── get_git_info()                               - now uses find_git_root()
```

## Testing

### Before Fix

```bash
$ cd /project/src/components
$ loct scan
[OK] Saved to ./.loctree/snapshot.json
# Missing branch@commit in path!

$ loct '.git_context'
{
  "repo": null,
  "branch": null,
  "commit": null,
  "scan_id": null
}
```

### After Fix

```bash
$ cd /project/src/components
$ loct scan
[OK] Saved to ./.loctree/main@abc1234/snapshot.json

$ loct '.git_context'
{
  "repo": "my-project",
  "branch": "main",
  "commit": "abc1234",
  "scan_id": "main@abc1234"
}
```

### Regression Tests

```rust
#[test]
fn test_find_git_root_from_nested_dir() {
    let (temp_dir, _repo) = create_test_repo();
    let path = temp_dir.path();

    // Create deeply nested directory structure
    let nested = path.join("src").join("deep").join("nested").join("dir");
    std::fs::create_dir_all(&nested).unwrap();

    // find_git_root should find the repo root from nested dir
    let root = find_git_root(&nested);
    assert!(root.is_some(), "Should find git root from nested directory");

    let expected = temp_dir.path().canonicalize().unwrap();
    let actual = root.unwrap().canonicalize().unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_find_git_root_non_git_dir() {
    let temp_dir = TempDir::new().unwrap();
    let root = find_git_root(temp_dir.path());
    assert!(root.is_none(), "Should return None for non-git directory");
}

#[test]
fn test_find_git_root_worktree() {
    let (main_dir, main_repo) = create_test_repo();
    let worktree_dir = TempDir::new().unwrap();

    // Create a worktree (this creates a .git file pointing to main repo)
    main_repo
        .worktree("test-worktree", worktree_dir.path(), None)
        .unwrap();

    // find_git_root should work from worktree
    let root = find_git_root(worktree_dir.path());
    assert!(root.is_some(), "Should find git root from worktree");

    let actual = root.unwrap().canonicalize().unwrap();
    let expected = worktree_dir.path().canonicalize().unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_find_git_root_nested_repo_chooses_closest() {
    let (outer_dir, _outer_repo) = create_test_repo();

    // Create an inner git repo inside the outer one
    let inner_path = outer_dir.path().join("packages").join("inner");
    std::fs::create_dir_all(&inner_path).unwrap();
    let inner_repo = Repository::init(&inner_path).unwrap();

    // Create a nested dir inside the inner repo
    let deep = inner_path.join("src").join("deep");
    std::fs::create_dir_all(&deep).unwrap();

    // find_git_root from deep should find inner repo, not outer
    let root = find_git_root(&deep);
    assert!(root.is_some(), "Should find git root");

    let actual = root.unwrap().canonicalize().unwrap();
    let expected = inner_path.canonicalize().unwrap();
    assert_eq!(actual, expected, "Should find closest (inner) repo, not outer");
}
```

## Impact

- Consistent git context detection regardless of working directory
- Proper `.gitignore` application from any subdirectory
- Correct `branch@commit` in snapshot paths
- Support for git worktrees and submodules

## Technical Notes

### Why libgit2 instead of shell commands?

1. **Proper upward search**: `Repository::discover()` walks up the directory tree
2. **Worktree support**: Handles `.git` files (not just directories)
3. **No subprocess overhead**: Native library call
4. **Already a dependency**: Used by `GitRepo` for blame/diff features

### Backwards Compatibility

The fix is transparent - existing workflows continue to work, but now also work correctly from subdirectories.

## Related

- Git context in snapshots: `loct '.git_context'`
- GitIgnoreChecker: `loctree_rs/src/fs_utils.rs`
- Snapshot path resolution: `loctree_rs/src/snapshot.rs`
