use crate::source::{Located, Location};
use std::{
    error::Error,
    fmt::{self, Display},
};

mod sealed {
    pub trait Sealed {}
}

pub trait LocatedError: sealed::Sealed {
    fn source(&self) -> &dyn Error;
    fn location(&self) -> &Location;
}

#[derive(Default)]
pub struct Diagnostics(Vec<Box<dyn 'static + LocatedError>>);

impl<E: 'static + LocatedError> From<E> for Diagnostics {
    fn from(error: E) -> Self {
        Diagnostics(vec![Box::new(error)])
    }
}

impl<E: 'static + LocatedError> From<Vec<E>> for Diagnostics {
    fn from(errors: Vec<E>) -> Self {
        let errors = errors
            .into_iter()
            .map(|error| {
                let errors: Box<dyn LocatedError> = Box::new(error);
                errors
            })
            .collect();

        Diagnostics(errors)
    }
}

impl Display for Diagnostics {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Diagnostics(errors) = self;
        if errors.is_empty() {
            return writeln!(fmt, "No errors were reported");
        }

        for error in errors {
            writeln!(fmt, "error: {}", error.source())?;

            let location = error.location();
            writeln!(fmt, " --> {}", location)?;

            //FIXME: Demasiado indecente
            let digits = location.end().line().to_string().chars().count();
            writeln!(fmt, "{:digits$} |", "", digits = digits)?;

            for line_number in location.start().line()..=location.end().line() {
                location.source().with_line(line_number, |line| {
                    writeln!(fmt, "{:>digits$} | {}", line_number, line, digits = digits)
                })?
            }

            let (from, to) = (location.start().column(), location.end().column() - 1);
            let min = from.min(to);
            let max = from.max(to);

            let skip = (min - 1) as usize;
            let highlight = (max - min + 1) as usize;

            writeln!(
                fmt,
                "{:digits$} | {:skip$}{:^<highlight$}",
                "",
                "",
                "",
                digits = digits,
                skip = skip,
                highlight = highlight
            )?;

            writeln!(fmt)?;
        }

        let error_or_errors = if errors.len() == 1 { "error" } else { "errors" };
        writeln!(
            fmt,
            "Build failed with {} {}",
            errors.len(),
            error_or_errors
        )
    }
}

impl<E: Error> sealed::Sealed for Located<E> {}

impl<E: Error> LocatedError for Located<E> {
    fn source(&self) -> &dyn Error {
        self.val()
    }

    fn location(&self) -> &Location {
        Located::location(self)
    }
}
