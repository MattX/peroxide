// Copyright 2018-2019 Matthieu Felix
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp::max;

/// Checks that a vector has at least `min`, at most `max` entries.
// TODO this is not really idiomatic and should probably be made to return a boolean
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

pub fn max_optional<T: Ord + Copy>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(max(a, b)),
        _ => a.or(b),
    }
}

pub fn parse_num(s: &str, base: u32) -> Result<i64, String> {
    if base > 36 {
        panic!("Invalid base {}.", base);
    }

    let mut r = 0 as i64;
    let mut it = s.chars().peekable();
    let reverse = it.peek() == Some(&'-');
    if reverse {
        it.next();
    }

    for d in it {
        let n = d.to_digit(base);
        if let Some(n) = n {
            r = r * i64::from(base) + i64::from(n);
        } else {
            return Err(format!("Invalid digit for base {}: {}", base, d));
        }
    }

    if reverse {
        r = -r;
    }
    Ok(r)
}

pub fn str_to_char_vec(s: &str) -> Vec<char> {
    s.chars().collect()
}

pub fn char_vec_to_str(v: &[char]) -> String {
    v.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_num() {
        assert_eq!(42, parse_num("101010", 2).unwrap());
        assert_eq!(42, parse_num("2a", 16).unwrap());
        assert_eq!(42, parse_num("42", 10).unwrap());
        assert_eq!(-15, parse_num("-F", 16).unwrap());
    }
}
