use cstree::build::GreenNodeBuilder;
use cstree::green::GreenNode;
use cstree::interning::Interner;
use logos::Lexer;
use logos::Logos;

use crate::expr_parser::parse_expr;
use crate::syntax::SyntaxKind;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\f]+")] // Ignore this regex pattern between tokens
pub enum Token {
    // instead of the expr token, we can use pg_query and merge afterwards
    // https://github.com/pganalyze/pg_query.rs/blob/0a893077643dd823c06a43ba4f68fb5bbc9b0d18/src/query.rs#L257
    #[regex("[a-zA-Z0-9_]+[^;]*;"gm)]
    Expr,
    #[regex("\n+"gm)]
    Newline,
    #[regex("/\\*[^*]*\\*+(?:[^/*][^*]*\\*+)*/|--[^\n]*"g)]
    Comment,
}

pub struct Parser<'input> {
    lexer: Lexer<'input, Token>,
    builder: GreenNodeBuilder<'static, 'static, SyntaxKind>,
    // the list of syntax errors we've accumulated so far
    errors: Vec<String>,
}

impl<'input> Parser<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            lexer: Token::lexer(input),
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<(), String> {
        self.builder.start_node(SyntaxKind::Root);
        self.parse_next_token();
        self.builder.finish_node();
        return Ok(());
    }

    fn parse_next_token(&mut self) {
        let token = self.lexer.next();
        println!("token: {:?}", token);
        match token {
            Some(Ok(token)) => {
                match token {
                    Token::Comment => {
                        self.builder.token(SyntaxKind::Comment, self.lexer.slice());
                    }
                    Token::Newline => {
                        self.builder.token(SyntaxKind::Newline, self.lexer.slice());
                    }
                    Token::Expr => {
                        parse_expr(self.lexer.slice(), &mut self.builder);
                    }
                };
                self.parse_next_token();
            }
            Some(Err(_)) => {
                self.errors
                    .push(format!("Error parsing token: '{:?}'", token));
                self.parse_next_token();
            }
            None => return,
        };
    }

    pub fn finish(mut self) -> (GreenNode, impl Interner) {
        // assert!(self.lexer.next().map(|t| t == Token::EoF).unwrap_or(true));
        let (tree, cache) = self.builder.finish();
        (tree, cache.unwrap().into_interner().unwrap())
    }
}

#[test]
fn test_lexer() {
    let input = "select * from contact where id = '123';\n\n-- test comment\n\nselect wrong statement;\n\nselect id,username from contact\n\nselect id,name\nfrom contact -- test inline comment\nwhere id = '123';\n\n";

    let mut lex = Token::lexer(&input);

    assert_eq!(lex.next(), Some(Ok(Token::Expr)));
    assert_eq!(lex.slice(), "select * from contact where id = '123';");

    assert_eq!(lex.next(), Some(Ok(Token::Newline)));

    assert_eq!(lex.next(), Some(Ok(Token::Comment)));
    assert_eq!(lex.slice(), "-- test comment");

    assert_eq!(lex.next(), Some(Ok(Token::Newline)));

    assert_eq!(lex.next(), Some(Ok(Token::Expr)));
    assert_eq!(lex.slice(), "select wrong statement;");

    assert_eq!(lex.next(), Some(Ok(Token::Newline)));

    assert_eq!(lex.next(), Some(Ok(Token::Expr)));
    assert_eq!(lex.slice(), "select id,username from contact\n\nselect id,name\nfrom contact -- test inline comment\nwhere id = '123';");
}