use crate::{ErrorInfo, Expr, LiteralType, Object, Span, TokenType};

pub trait Visitor {
    fn visit_assign_expr(
        &mut self,
        name: &String,
        value: &Box<Expr>,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_binary_expr(
        &mut self,
        left: &Box<Expr>,
        op: &TokenType,
        right: &Box<Expr>,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_call_expr(
        &mut self,
        callee: &Box<Expr>,
        paren: &TokenType,
        args: &Vec<Expr>,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_get_expr(
        &mut self,
        object: &Box<Expr>,
        name: &String,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_grouping_expr(&mut self, expr: &Box<Expr>, span: &Span) -> Result<Object, ErrorInfo>;
    fn visit_literal_expr(&mut self, value: &LiteralType, span: &Span)
        -> Result<Object, ErrorInfo>;
    fn visit_logical_expr(
        &mut self,
        left: &Box<Expr>,
        op: &TokenType,
        right: &Box<Expr>,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_set_expr(
        &mut self,
        object: &Box<Expr>,
        name: &String,
        value: &Box<Expr>,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_super_expr(&mut self, name: &String, span: &Span) -> Result<Object, ErrorInfo>;
    fn visit_unary_expr(
        &mut self,
        op: &TokenType,
        right: &Box<Expr>,
        span: &Span,
    ) -> Result<Object, ErrorInfo>;
    fn visit_variable_expr(&mut self, name: &String, span: &Span) -> Result<Object, ErrorInfo>;
}
