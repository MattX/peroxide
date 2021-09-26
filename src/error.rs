use std::fmt::Write;
use std::iter::repeat;

use value::Locator;

pub fn locate_message(source: &str, locator: &Locator, msg: &str) -> String {
    let mut output = String::new();

    let max_num_line_width = locator.range.end.0.to_string().chars().count();
    let prefix: String = repeat(' ').take(max_num_line_width).collect();

    writeln!(output, "error: {}", msg).unwrap();
    writeln!(output, "{}--> {}", prefix, locator).unwrap();
    writeln!(output, "{} |", prefix).unwrap();

    for (i_line, line) in source.lines().enumerate() {
        if i_line + 1 < locator.range.start.0 as usize {
            continue;
        } else if i_line + 1 > locator.range.end.0 as usize {
            break;
        }
        if i_line + 1 == locator.range.start.0 as usize {
            writeln!(
                output,
                "{:width$} |   {}",
                i_line + 1,
                line,
                width = max_num_line_width
            )
            .unwrap();
            let marker = if i_line + 1 == locator.range.end.0 as usize {
                let underline = if locator.range.start.1 == locator.range.end.1 {
                    "^".to_string()
                } else {
                    "^".to_string()
                        + &repeat('-')
                            .take((locator.range.end.1 - locator.range.start.1 - 1) as usize)
                            .collect::<String>()
                        + "^"
                };
                let prefix = repeat(' ')
                    .take((locator.range.start.1 + 1) as usize)
                    .collect::<String>();
                prefix + &underline
            } else {
                repeat('-')
                    .take((locator.range.start.1 + 2) as usize)
                    .collect::<String>()
                    + "^"
            };
            writeln!(output, "{} | {}", prefix, marker).unwrap();
        } else if i_line + 1 == locator.range.end.0 as usize {
            writeln!(
                output,
                "{:width$} | | {}",
                i_line + 1,
                line,
                width = max_num_line_width
            )
            .unwrap();
            let marker = repeat('-')
                .take((locator.range.end.1 + 2) as usize)
                .collect::<String>()
                + "^";
            writeln!(output, "{} | {}", prefix, marker).unwrap();
        } else {
            writeln!(
                output,
                "{:width$} | | {}",
                i_line + 1,
                line,
                width = max_num_line_width
            )
            .unwrap();
        }
    }

    writeln!(output, "{} |", prefix).unwrap();
    output
}
