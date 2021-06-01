use compiler::{
    error::Diagnostics,
    lex::Lexer,
    parse,
    source::{self, SourceName},
};

fn main() {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();

    let lexer = Lexer::new(source::consume(&mut stdin), SourceName::from("<stdin>"));
    let diagnostics = match lexer.try_exhaustive() {
        Err(errors) => Diagnostics::from(errors),
        Ok(tokens) => {
            print!("Tokens: {:#?}\n\n", tokens);

            match parse::parse(tokens.iter()) {
                Err(error) => Diagnostics::from(error),
                Ok(ast) => {
                    println!("{:#?}", ast);
                    Diagnostics::default()
                }
            }
        }
    };

    eprint!("{}", diagnostics);
}
