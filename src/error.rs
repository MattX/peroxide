use std::fmt::Write;
use std::rc::Rc;

use ast::MacroSource;
use lex::CodeRange;
use value::Locator;
use File;

/// An error which can provide a code location
trait SourcedError: std::error::Error {
    fn code_source(&self) -> Option<&Source>;
}

#[derive(Debug, Clone)]
pub enum Source {
    /// Directly from source code
    Code(SourceFileLocator),
    /// Generated by a macro (defined in `macro_source`) from code originating in `code_source`.
    Macro {
        macro_source: Rc<MacroSource>,
        code_source: Box<Source>,
    },
}

#[derive(Debug, Clone)]
pub struct SourceFileLocator {
    pub file: Rc<File>,
    pub range: CodeRange,
}

pub fn locate_message(locator: &Locator, msg: &str) -> String {
    let mut output = String::new();

    let max_num_line_width = locator.range.end.0.to_string().chars().count();
    let prefix: String = " ".repeat(max_num_line_width);

    writeln!(output, "error: {}", msg).unwrap();
    writeln!(output, "{}--> {}", prefix, locator).unwrap();
    writeln!(output, "{} |", prefix).unwrap();

    for (i_line, line) in locator.file.source.lines().enumerate() {
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
                let underline =
                    "^".repeat((locator.range.end.1 - locator.range.start.1 + 1) as usize);
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
