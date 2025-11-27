pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut d = vec![vec![0; b_len + 1]; a_len + 1];

    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in d[0].iter_mut().enumerate() {
        *val = j;
    }

    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            d[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(d[i][j + 1] + 1, d[i + 1][j] + 1),
                d[i][j] + cost,
            );
        }
    }

    d[a_len][b_len]
}

pub fn similarity(a: &str, b: &str) -> f64 {
    let dist = levenshtein(a, b);
    let max_len = std::cmp::max(a.chars().count(), b.chars().count());
    if max_len == 0 {
        1.0
    } else {
        1.0 - (dist as f64 / max_len as f64)
    }
}
