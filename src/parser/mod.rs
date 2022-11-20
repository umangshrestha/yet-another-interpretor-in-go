use crate::ast::Program;
use crate::Error;
use crate::ErrorInfo;
use crate::Lexer;
use crate::Span;
use crate::{Expr, LiteralType, Stmt};
use crate::{TokenInfo, TokenType};

pub struct Parser {
    lexer: Lexer,
    prev: TokenInfo,
    curr: TokenInfo,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Self {
        Self {
            prev: TokenInfo::new(TokenType::Eof, 0, 0, 0),
            curr: lexer.next(),
            lexer,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ErrorInfo> {
        let mut stmt = Vec::new();
        while !self.curr.is(TokenType::Eof) {
            stmt.push(self.declaration()?);
        }

        Ok(Program::new(stmt))
    }

    fn declaration(&mut self) -> Result<Stmt, ErrorInfo> {
        match self.curr.token {
            TokenType::Let | TokenType::Const => self.let_declaration(),
            TokenType::Class => self.class_declaration(),
            TokenType::Function => {
                self.advance();
                self.function_declaration()
            }
            _ => self.statement(),
        }
    }

    fn let_declaration(&mut self) -> Result<Stmt, ErrorInfo> {
        let is_const = self.curr.is(TokenType::Const);
        self.advance();
        let (name, span) = self.get_identifier()?;
        let mut value = None;
        if self.curr.is(TokenType::Assign) {
            self.advance();
            value = Some(self.expression()?);
        }
        self.should_be(TokenType::Semicolon)?;
        Ok(Stmt::Let {
            name,
            value,
            is_const,
            span,
        })
    }

    fn class_declaration(&mut self) -> Result<Stmt, ErrorInfo> {
        let (name, span) = self.get_identifier()?;
        let super_class = if self.curr.is(TokenType::Lt) {
            self.advance();
            let (super_class_name, span) = self.get_identifier()?;
            if name == super_class_name {
                let error = Error::Parse("Cannot inherit from itself".to_string());
                return Err(ErrorInfo::new_with_span(error, span));
            }
            Some(super_class_name)
        } else {
            None
        };
        self.should_be(TokenType::LBrace)?;
        let mut methods = Vec::new();
        while !self.curr.is(TokenType::RBrace) && !self.curr.is(TokenType::Eof) {
            methods.push(self.function_declaration()?);
        }
        self.should_be(TokenType::RBrace)?;
        Ok(Stmt::Class {
            name,
            super_class,
            methods,
            span,
        })
    }

    fn function_declaration(&mut self) -> Result<Stmt, ErrorInfo> {
        let (name, span) = self.get_identifier()?;
        self.should_be(TokenType::LParen)?;
        let mut params = Vec::new();
        if !self.curr.is(TokenType::RParen) {
            let (param, _) = self.get_identifier()?;
            params.push(param);
            while self.curr.is(TokenType::Comma) {
                self.advance();
                let (param, _) = self.get_identifier()?;
                params.push(param);
            }
        }
        self.should_be(TokenType::RParen)?;
        let body = self.block_statement()?;
        Ok(Stmt::Function {
            name,
            params: params,
            body: Box::new(body),
            span,
        })
    }

    fn statement(&mut self) -> Result<Stmt, ErrorInfo> {
        match self.curr.token {
            TokenType::Print => self.print_statement(),
            TokenType::If => self.if_statement(),
            TokenType::While => self.while_statement(),
            TokenType::For => self.for_statement(),
            TokenType::Return => self.return_statement(),
            TokenType::LCurly => self.block_statement(),
            _ => self.expression_statement(),
        }
    }

    fn expression_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let span = self.curr.span.clone();
        let expr = self.expression()?;
        self.should_be(TokenType::Semicolon)?;
        Ok(Stmt::Expr { expr, span })
    }

    fn print_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let (_, span) = self.advance();
        let expr = self.expression()?;
        self.should_be(TokenType::Semicolon)?;
        Ok(Stmt::Print { expr, span })
    }

    fn return_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let (_, span) = self.advance();
        let mut value = None;
        if !self.curr.is(TokenType::Semicolon) {
            value = Some(self.expression()?);
        }
        self.should_be(TokenType::Semicolon)?;
        Ok(Stmt::Return { value, span })
    }

    fn for_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let (_, span) = self.advance();
        self.should_be(TokenType::LParen)?;
        let initializer = match self.curr.token {
            TokenType::Semicolon => None,
            TokenType::Let => Some(Box::new(self.let_declaration()?)),
            _ => Some(Box::new(self.expression_statement()?)),
        };

        let condition = match self.curr.token {
            TokenType::Semicolon => None,
            _ => Some(self.expression()?),
        };
        self.should_be(TokenType::Semicolon)?;

        let increment = match self.curr.token {
            TokenType::RParen => None,
            _ => Some(self.expression()?),
        };
        self.should_be(TokenType::RParen)?;

        let body = self.statement()?;

        Ok(Stmt::For {
            increment,
            condition,
            initializer,
            body: Box::new(body),
            span,
        })
    }

    fn if_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let (_, span) = self.advance();
        self.should_be(TokenType::LParen)?;
        let condition = self.expression()?;
        self.should_be(TokenType::RParen)?;
        let truthy = Box::new(self.statement()?);
        let mut falsy = None;
        if self.curr.is(TokenType::Else) {
            self.advance();
            falsy = Some(Box::new(self.statement()?));
        }
        Ok(Stmt::If {
            condition,
            truthy,
            falsy,
            span,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let (_, span) = self.advance();
        self.should_be(TokenType::LParen)?;
        let condition = self.expression()?;
        self.should_be(TokenType::RParen)?;
        let body = Box::new(self.statement()?);
        Ok(Stmt::While {
            condition,
            body,
            span,
        })
    }

    fn block_statement(&mut self) -> Result<Stmt, ErrorInfo> {
        let span = self.should_be(TokenType::LCurly)?;
        let mut stmt = Vec::new();
        while !self.curr.is(TokenType::RCurly) && !self.curr.is(TokenType::Eof) {
            stmt.push(self.declaration()?);
        }
        self.should_be(TokenType::RCurly)?;
        Ok(Stmt::Block { stmt, span })
    }
}

impl Parser {
    fn expression(&mut self) -> Result<Expr, ErrorInfo> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ErrorInfo> {
        let left = self.or()?;
        if let TokenType::Assign
        | TokenType::PlusEq
        | TokenType::SubEq
        | TokenType::ModEq
        | TokenType::DivEq
        | TokenType::AndEq
        | TokenType::OrEq
        | TokenType::MulEq
        | TokenType::XorEq = self.curr.token
        {
            let (_, span) = self.advance();
            let right = self.or()?;
            return match left {
                Expr::Variable { name, span } => Ok(Expr::Assign {
                    name,
                    value: Box::new(right),
                    span,
                }),
                Expr::Get { object, name, span } => Ok(Expr::Set {
                    object,
                    name,
                    value: Box::new(right),
                    span,
                }),
                _ => {
                    let error = Error::Parse("Invalid assignment target".to_string());
                    return Err(ErrorInfo::new_with_span(error, span));
                }
            };
        }

        Ok(left)
    }

    fn or(&mut self) -> Result<Expr, ErrorInfo> {
        let mut left = self.and()?;
        while self.curr.is(TokenType::Or) {
            let (op, span) = self.advance();
            let right = self.and()?;
            left = Expr::Logical {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, ErrorInfo> {
        let mut left = self.equality()?;
        while self.curr.is(TokenType::LAnd) {
            let (op, span) = self.advance();
            let right = self.equality()?;
            left = Expr::Logical {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn equality(&mut self) -> Result<Expr, ErrorInfo> {
        let mut left = self.comparison()?;
        while let TokenType::Eq | TokenType::Ne = self.curr.token {
            let (op, span) = self.advance();
            let right = self.comparison()?;
            left = Expr::Logical {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, ErrorInfo> {
        let mut left = self.term()?;
        while let TokenType::Gt | TokenType::Gte | TokenType::Lt | TokenType::Lte = self.curr.token
        {
            let (op, span) = self.advance();
            let right = self.term()?;
            left = Expr::Logical {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, ErrorInfo> {
        let mut left = self.factor()?;
        while let TokenType::Plus
        | TokenType::Minus
        | TokenType::Or
        | TokenType::And
        | TokenType::Xor = self.curr.token
        {
            let (op, span) = self.advance();
            let right = self.factor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, ErrorInfo> {
        let mut left = self.unary()?;
        while let TokenType::Times | TokenType::Divide = self.curr.token {
            let (op, span) = self.advance();
            let right = self.unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, ErrorInfo> {
        if let TokenType::Minus | TokenType::Not | TokenType::Plus = self.curr.token {
            let (op, span) = self.advance();
            let right = self.unary()?;
            Ok(Expr::Unary {
                op,
                right: Box::new(right),
                span,
            })
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Result<Expr, ErrorInfo> {
        let mut expr = self.primary()?;
        loop {
            match self.curr.token {
                TokenType::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.curr.is(TokenType::RParen) {
                        loop {
                            args.push(self.expression()?);
                            if !self.curr.is(TokenType::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                }
                TokenType::Dot => {
                    self.advance();
                    let (name, span) = self.get_identifier()?;
                    expr = Expr::Get {
                        object: Box::new(expr),
                        name,
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, ErrorInfo> {
        let tok = self.curr.clone();
        let span = tok.span;
        match tok.token {
            TokenType::True => {
                self.advance();
                let value = LiteralType::Boolean(true);
                Ok(Expr::Literal { value, span })
            }
            TokenType::False => {
                self.advance();
                let value = LiteralType::Boolean(false);
                Ok(Expr::Literal { value, span })
            }
            TokenType::Number(x) => {
                self.advance();
                let value = LiteralType::Number(x);
                Ok(Expr::Literal { value, span })
            }
            TokenType::String(x) => {
                self.advance();
                let value = LiteralType::String(x);
                Ok(Expr::Literal { value, span })
            }
            TokenType::Identifier(name) => {
                self.advance();
                Ok(Expr::Variable { name, span })
            }
            TokenType::LParen => {
                self.advance();
                let expr = Box::new(self.expression()?);
                self.should_be(TokenType::RParen)?;
                Ok(Expr::Grouping { expr, span })
            }
            TokenType::Super => {
                self.should_be(TokenType::Dot)?;
                let (name, span) = self.get_identifier()?;
                Ok(Expr::Super { name, span })
            }
            TokenType::This => {
                let name = "this".to_string();
                Ok(Expr::Variable { name, span })
            }
            _ => {
                let error = Error::Parse("Expect expression.".to_string());
                Err(ErrorInfo::new_with_span(error, span))
            }
        }
    }
}

impl Parser {
    fn should_be(&mut self, token_type: TokenType) -> Result<Span, ErrorInfo> {
        let (val, span) = self.advance();
        if val == token_type {
            Ok(span)
        } else {
            let error = Error::Syntax(format!(
                "Expected: \"{}\" Found: \"{}\"",
                token_type, self.curr.token
            ));
            Err(ErrorInfo::new_with_span(error, span))
        }
    }

    fn get_identifier(&mut self) -> Result<(String, Span), ErrorInfo> {
        let (val, span) = self.advance();
        if let TokenType::Identifier(name) = val {
            Ok((name, span))
        } else {
            let error = Error::Syntax(format!("Expected: \"Identifier\" Found: \"{}\"", val));
            Err(ErrorInfo::new_with_span(error, span))
        }
    }

    fn advance(&mut self) -> (TokenType, Span) {
        self.prev = self.curr.clone();
        self.curr = self.lexer.next();
        (self.prev.token.clone(), self.prev.span.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_unary() {
        let input = "-(1 / (2 * 32));";
        let mut parser = Parser::new(Lexer::new(input.into()));
        let expr = parser.parse_program().unwrap();
        assert_eq!(expr.to_string(), "((- (/ 1 (* 2 32))))");
    }

    #[test]
    fn test_assignment() {
        let input = "
        let a = 1;
        print a ;";
        let mut parser = Parser::new(Lexer::new(input.to_string()));
        let expr = parser.parse_program().unwrap();
        assert_eq!(expr.to_string(), "((let a 1)(print a))");
    }

    #[test]
    fn test_if() {
        let input = "
        if (a == 1) {
            print a;
        } else {
            print b;
        }";
        let mut parser = Parser::new(Lexer::new(input.to_string()));
        let expr = parser.parse_program().unwrap();
        assert_eq!(
            expr.to_string(),
            "((if (== a 1) then ((print a)) else ((print b))))"
        );
    }
}
