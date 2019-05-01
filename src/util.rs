/// Checks that a vector has at least `min`, at most `max` entries.
pub fn check_len<T>(v: &[T], min: Option<usize>, max: Option<usize>) -> Result<(), String> {
    if let Some(m) = min {
        if v.len() < m {
            return Err(format!("Too few values, expecting at least {}.", m));
        }
    };
    if let Some(m) = max {
        if v.len() > m {
            return Err(format!("Too many values, expecting at most {}.", m));
        }
    };
    Ok(())
}

/// Extracts a single element from a length 1 vector, or fails.
pub fn extract_single<T>(mut v: Vec<T>) -> Result<T, String> {
    if v.len() > 1 {
        return Err(format!("Too many values, expecting 1 and got {}.", v.len()));
    }
    v.pop()
        .ok_or_else(|| "Too few values, expecting one and got 0.".into())
}
