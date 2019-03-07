/// Checks that a vector has at least `min`, at most `max` entries.
pub fn with_check_len<T>(
    v: Vec<T>,
    min: Option<usize>,
    max: Option<usize>,
) -> Result<Vec<T>, String> {
    if let Some(m) = min {
        if v.len() < m {
            return Err(format!("Too few values, expecting at least {}", m));
        }
    };
    if let Some(m) = max {
        if v.len() > m {
            return Err(format!("Too many values, expecting at most {}", m));
        }
    };
    Ok(v)
}
