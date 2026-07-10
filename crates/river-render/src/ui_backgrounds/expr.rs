//! Minimal math expression evaluator used to drive background parameters
//! from plugin-authored KDL, e.g. `y = "sin(time * speed + i * 0.3) * 40 + 200"`.
//!
//! Supports: + - * / %, unary -, parentheses, numeric literals, and a fixed
//! set of variables/functions supplied per-evaluation via `Env`.
//!
//! This is intentionally small and dependency-free. It is NOT a general
//! scripting language — no branching, no user-defined functions, no loops.
//! That's the point: it's fast, sandboxed, and can't do anything unsafe.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ExprError {
    UnexpectedChar(char),
    UnexpectedEnd,
    UnknownVar(String),
    UnknownFn(String),
    WrongArgCount { func: String, expected: usize, got: usize },
}

impl std::fmt::Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprError::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            ExprError::UnexpectedEnd => write!(f, "unexpected end of expression"),
            ExprError::UnknownVar(v) => write!(f, "unknown variable '{v}'"),
            ExprError::UnknownFn(v) => write!(f, "unknown function '{v}'"),
            ExprError::WrongArgCount { func, expected, got } => {
                write!(f, "'{func}' expects {expected} arg(s), got {got}")
            }
        }
    }
}
impl std::error::Error for ExprError {}

/// A parsed, reusable expression. Parse once (e.g. at plugin load time),
/// evaluate every frame — parsing is the expensive part, evaluation is cheap.
#[derive(Debug, Clone)]
pub struct Expr(Node);

#[derive(Debug, Clone)]
enum Node {
    Num(f64),
    Var(String),
    Neg(Box<Node>),
    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),
    Mod(Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}

/// Variables/constants available inside an expression at eval time.
/// Cheap to construct per-element-per-frame (just a small map of f64s).
#[derive(Default, Clone)]
pub struct Env {
    vars: HashMap<String, f64>,
}

impl Env {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set(mut self, name: &str, value: f64) -> Self {
        self.vars.insert(name.to_string(), value);
        self
    }
    pub fn insert(&mut self, name: &str, value: f64) {
        self.vars.insert(name.to_string(), value);
    }
}

impl Expr {
    pub fn parse(src: &str) -> Result<Self, ExprError> {
        let tokens = tokenize(src)?;
        let mut p = Parser { tokens, pos: 0 };
        let node = p.parse_expr()?;
        if p.pos != p.tokens.len() {
            return Err(ExprError::UnexpectedChar('?'));
        }
        Ok(Expr(node))
    }

    /// Convenience: parse a constant-or-expression. Falls back to a bare
    /// number fast-path so plugin authors don't pay parser overhead for
    /// simple literal values.
    pub fn parse_or_const(src: &str) -> Result<Self, ExprError> {
        if let Ok(n) = src.trim().parse::<f64>() {
            return Ok(Expr(Node::Num(n)));
        }
        Self::parse(src)
    }

    pub fn eval(&self, env: &Env) -> Result<f64, ExprError> {
        eval_node(&self.0, env)
    }

    /// Recursively checks if the expression references the 'time' variable.
    pub fn references_time(&self) -> bool {
        fn node_references_time(node: &Node) -> bool {
            match node {
                Node::Var(name) => name == "time",
                Node::Neg(a) => node_references_time(a),
                Node::Add(a, b) | Node::Sub(a, b) | Node::Mul(a, b) | Node::Div(a, b) | Node::Mod(a, b) => {
                    node_references_time(a) || node_references_time(b)
                }
                Node::Call(_, args) => args.iter().any(node_references_time),
                _ => false,
            }
        }
        node_references_time(&self.0)
    }
}

fn eval_node(node: &Node, env: &Env) -> Result<f64, ExprError> {
    Ok(match node {
        Node::Num(n) => *n,
        Node::Var(name) => {
            if name == "pi" {
                std::f64::consts::PI
            } else if name == "tau" {
                std::f64::consts::TAU
            } else {
                *env
                    .vars
                    .get(name)
                    .ok_or_else(|| ExprError::UnknownVar(name.clone()))?
            }
        }
        Node::Neg(a) => -eval_node(a, env)?,
        Node::Add(a, b) => eval_node(a, env)? + eval_node(b, env)?,
        Node::Sub(a, b) => eval_node(a, env)? - eval_node(b, env)?,
        Node::Mul(a, b) => eval_node(a, env)? * eval_node(b, env)?,
        Node::Div(a, b) => {
            let d = eval_node(b, env)?;
            if d == 0.0 { 0.0 } else { eval_node(a, env)? / d }
        }
        Node::Mod(a, b) => {
            let d = eval_node(b, env)?;
            if d == 0.0 { 0.0 } else { eval_node(a, env)?.rem_euclid(d) }
        }
        Node::Call(name, args) => {
            let vals: Result<Vec<f64>, ExprError> = args.iter().map(|a| eval_node(a, env)).collect();
            let vals = vals?;
            call_fn(name, &vals)?
        }
    })
}

fn call_fn(name: &str, args: &[f64]) -> Result<f64, ExprError> {
    macro_rules! need {
        ($n:expr) => {
            if args.len() != $n {
                return Err(ExprError::WrongArgCount { func: name.to_string(), expected: $n, got: args.len() });
            }
        };
    }
    Ok(match name {
        "sin" => { need!(1); args[0].sin() }
        "cos" => { need!(1); args[0].cos() }
        "tan" => { need!(1); args[0].tan() }
        "abs" => { need!(1); args[0].abs() }
        "sqrt" => { need!(1); args[0].max(0.0).sqrt() }
        "floor" => { need!(1); args[0].floor() }
        "ceil" => { need!(1); args[0].ceil() }
        "round" => { need!(1); args[0].round() }
        "min" => { need!(2); args[0].min(args[1]) }
        "max" => { need!(2); args[0].max(args[1]) }
        "clamp" => { need!(3); args[0].clamp(args[1].min(args[2]), args[1].max(args[2])) }
        "lerp" => { need!(3); args[0] + (args[1] - args[0]) * args[2] }
        "pow" => { need!(2); args[0].powf(args[1]) }
        "wrap" => {
            // wrap(x, max) -> x mod max, always positive
            need!(2);
            if args[1] == 0.0 { 0.0 } else { args[0].rem_euclid(args[1]) }
        }
        _ => return Err(ExprError::UnknownFn(name.to_string())),
    })
}

// ---------- tokenizer ----------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LParen,
    RParen,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<Tok>, ExprError> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' | '\r' => { i += 1; }
            '+' => { out.push(Tok::Plus); i += 1; }
            '-' => { out.push(Tok::Minus); i += 1; }
            '*' => { out.push(Tok::Star); i += 1; }
            '/' => { out.push(Tok::Slash); i += 1; }
            '%' => { out.push(Tok::Percent); i += 1; }
            '(' => { out.push(Tok::LParen); i += 1; }
            ')' => { out.push(Tok::RParen); i += 1; }
            ',' => { out.push(Tok::Comma); i += 1; }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n: f64 = s.parse().map_err(|_| ExprError::UnexpectedChar(c))?;
                out.push(Tok::Num(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push(Tok::Ident(s));
            }
            other => return Err(ExprError::UnexpectedChar(other)),
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }
    fn bump(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    // expr := term (('+'|'-') term)*
    fn parse_expr(&mut self) -> Result<Node, ExprError> {
        let mut node = self.parse_term()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => { self.bump(); node = Node::Add(Box::new(node), Box::new(self.parse_term()?)); }
                Some(Tok::Minus) => { self.bump(); node = Node::Sub(Box::new(node), Box::new(self.parse_term()?)); }
                _ => break,
            }
        }
        Ok(node)
    }

    // term := unary (('*'|'/'|'%') unary)*
    fn parse_term(&mut self) -> Result<Node, ExprError> {
        let mut node = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Tok::Star) => { self.bump(); node = Node::Mul(Box::new(node), Box::new(self.parse_unary()?)); }
                Some(Tok::Slash) => { self.bump(); node = Node::Div(Box::new(node), Box::new(self.parse_unary()?)); }
                Some(Tok::Percent) => { self.bump(); node = Node::Mod(Box::new(node), Box::new(self.parse_unary()?)); }
                _ => break,
            }
        }
        Ok(node)
    }

    // unary := '-' unary | primary
    fn parse_unary(&mut self) -> Result<Node, ExprError> {
        if let Some(Tok::Minus) = self.peek() {
            self.bump();
            return Ok(Node::Neg(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    // primary := NUM | IDENT('(' args ')')? | '(' expr ')'
    fn parse_primary(&mut self) -> Result<Node, ExprError> {
        match self.bump() {
            Some(Tok::Num(n)) => Ok(Node::Num(n)),
            Some(Tok::Ident(name)) => {
                if let Some(Tok::LParen) = self.peek() {
                    self.bump(); // consume (
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        args.push(self.parse_expr()?);
                        while matches!(self.peek(), Some(Tok::Comma)) {
                            self.bump();
                            args.push(self.parse_expr()?);
                        }
                    }
                    match self.bump() {
                        Some(Tok::RParen) => {}
                        _ => return Err(ExprError::UnexpectedEnd),
                    }
                    Ok(Node::Call(name, args))
                } else {
                    Ok(Node::Var(name))
                }
            }
            Some(Tok::LParen) => {
                let node = self.parse_expr()?;
                match self.bump() {
                    Some(Tok::RParen) => Ok(node),
                    _ => Err(ExprError::UnexpectedEnd),
                }
            }
            Some(other) => Err(ExprError::UnexpectedChar(format!("{other:?}").chars().next().unwrap_or('?'))),
            None => Err(ExprError::UnexpectedEnd),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_math() {
        let e = Expr::parse("2 + 3 * 4").unwrap();
        assert_eq!(e.eval(&Env::new()).unwrap(), 14.0);
    }

    #[test]
    fn vars_and_fns() {
        let e = Expr::parse("sin(time * speed) * 40 + 200").unwrap();
        let env = Env::new().set("time", 0.0).set("speed", 1.0);
        assert_eq!(e.eval(&env).unwrap(), 200.0);
    }

    #[test]
    fn clamp_lerp() {
        let e = Expr::parse("lerp(0, 100, 0.5)").unwrap();
        assert_eq!(e.eval(&Env::new()).unwrap(), 50.0);
    }

    #[test]
    fn unknown_var_errors() {
        let e = Expr::parse("i * 2").unwrap();
        assert!(e.eval(&Env::new()).is_err());
    }
}
