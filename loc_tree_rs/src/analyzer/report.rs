#[derive(Clone)]
pub struct CommandGap {
    pub name: String,
    pub locations: Vec<(String, usize)>,
}

#[derive(Clone)]
pub struct GraphData {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String, String)>, // from, to, kind
}

#[derive(Clone)]
pub struct RankedDup {
    pub name: String,
    pub files: Vec<String>,
    pub score: usize,
    pub prod_count: usize,
    pub dev_count: usize,
    pub canonical: String,
    pub refactors: Vec<String>,
}

pub struct ReportSection {
    pub root: String,
    pub files_analyzed: usize,
    pub ranked_dups: Vec<RankedDup>,
    pub cascades: Vec<(String, String)>,
    pub dynamic: Vec<(String, Vec<String>)>,
    pub analyze_limit: usize,
    pub missing_handlers: Vec<CommandGap>,
    pub unused_handlers: Vec<CommandGap>,
    pub command_counts: (usize, usize),
    pub open_base: Option<String>,
    pub graph: Option<GraphData>,
}
