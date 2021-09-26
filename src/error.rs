use std::fmt::Write;

use value::Locator;

pub fn locate_message(source: &str, locator: &Locator, msg: &str) -> String {
    let mut output = String::new();

    let max_num_line_width = locator.range.end.0.to_string().chars().count();
    let prefix: String = " ".repeat(max_num_line_width);

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
                        + &"-".repeat((locator.range.end.1 - locator.range.start.1 - 1) as usize)
                        + "^"
                };
                let prefix = " ".repeat((locator.range.start.1 + 1) as usize);
                prefix + &underline
            } else {
                "-".repeat((locator.range.start.1 + 2) as usize) + "^"
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
            let marker = "-".repeat((locator.range.end.1 + 2) as usize) + "^";
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
