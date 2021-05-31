use compiler::{
    lex::Lexer,
    parse,
    source::{self, SourceName},
};

fn main() {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();

    let lexer = Lexer::new(source::consume(&mut stdin), SourceName::from("<stdin>"));
    match lexer.try_exhaustive() {
        Err(errors) => eprintln!("{:#?}", errors),
        Ok(tokens) => {
            println!("Tokens: {:#?}", tokens);
            println!();
            println!("{:#?}", parse::parse(tokens.into_iter()));
        }
    }
}
