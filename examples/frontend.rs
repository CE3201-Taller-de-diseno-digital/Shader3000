use compiler::{error::Diagnostics, lex::Lexer, parse, source};

fn main() {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();

    let (start, stream) = source::consume(&mut stdin, "<stdin>");
    let lexer = Lexer::new(start.clone(), stream);

    let diagnostics = match lexer.try_exhaustive() {
        Err(errors) => Diagnostics::from(errors).kind("Lexical error"),

        Ok(tokens) => {
            print!("Tokens: {:#?}\n\n", tokens);

            match parse::parse(tokens.iter(), start) {
                Err(error) => Diagnostics::from(error).kind("Syntax error"),

                Ok(ast) => {
                    print!("Ast: {:#?}\n\n", ast);

                    match ast.resolve() {
                        Err(error) => Diagnostics::from(error).kind("Semantic error"),

                        Ok(ir) => {
                            println!("IR: {:#?}", ir);
                            Diagnostics::default()
                        }
                    }
                }
            }
        }
    };

    eprint!("{}", diagnostics);
}
