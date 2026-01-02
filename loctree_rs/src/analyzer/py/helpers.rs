//! Byte manipulation helper functions for Python parsing.
//!
//! Created by M&K (c)2025 The LibraxisAI Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

/// Helper to safely compare bytes at position with a keyword.
/// Returns true if the bytes at position match the keyword.
#[inline]
pub(super) fn bytes_match_keyword(bytes: &[u8], pos: usize, keyword: &[u8]) -> bool {
    if pos + keyword.len() > bytes.len() {
        return false;
    }
    &bytes[pos..pos + keyword.len()] == keyword
}

/// Helper to safely extract an ASCII identifier from bytes.
/// Returns the identifier as a string if valid ASCII, empty string otherwise.
#[inline]
pub(super) fn extract_ascii_ident(bytes: &[u8], start: usize, end: usize) -> String {
    if start >= end || end > bytes.len() {
        return String::new();
    }
    // Only extract if all bytes are valid ASCII identifier chars
    let slice = &bytes[start..end];
    if slice.iter().all(|b| b.is_ascii()) {
        String::from_utf8_lossy(slice).into_owned()
    } else {
        String::new()
    }
}

/// Python keywords and builtins to skip when detecting identifiers.
pub(super) const SKIP_BUILTINS: &[&str] = &[
    "None",
    "True",
    "False",
    "str",
    "int",
    "float",
    "bool",
    "bytes",
    "list",
    "dict",
    "set",
    "tuple",
    "frozenset",
    "type",
    "object",
    "Any",
    "Union",
    "Optional",
    "List",
    "Dict",
    "Set",
    "Tuple",
    "Callable",
    "Sequence",
    "Mapping",
    "Iterable",
    "Iterator",
    "Type",
    "self",
    "cls",
];

/// Extended skip list for type hint extraction.
pub(super) const SKIP_TYPE_HINTS: &[&str] = &[
    "None",
    "True",
    "False",
    "str",
    "int",
    "float",
    "bool",
    "bytes",
    "list",
    "dict",
    "set",
    "tuple",
    "frozenset",
    "type",
    "object",
    "Any",
    "Union",
    "Optional",
    "List",
    "Dict",
    "Set",
    "Tuple",
    "Callable",
    "Sequence",
    "Mapping",
    "Iterable",
    "Iterator",
    "Generator",
    "Coroutine",
    "Awaitable",
    "AsyncIterator",
    "AsyncGenerator",
    "Type",
    "ClassVar",
    "Final",
    "Literal",
    "TypeVar",
    "Generic",
    "Protocol",
    "Self",
    "self",
    "cls",
];

/// Python keywords that look like function calls but aren't.
pub(super) const PYTHON_KEYWORDS: &[&str] = &[
    "if", "else", "elif", "while", "for", "try", "except", "finally", "with", "as", "def", "class",
    "return", "yield", "raise", "import", "from", "pass", "break", "continue", "lambda", "and",
    "or", "not", "in", "is", "True", "False", "None", "assert", "del", "exec", "print", "global",
    "nonlocal", "async", "await",
];

/// Known type containers that take types as parameters.
pub(super) const TYPE_FACTORIES: &[&str] = &[
    "defaultdict",
    "Counter",
    "deque",
    "OrderedDict",
    "ChainMap",
    "namedtuple",
    "TypedDict",
    "NewType",
    "cast",
    // FastAPI dependency injection
    "Depends",
    "Security",
    // Pydantic
    "Field",
];
