//! Formula expressions over record metadata.
//!
//! The grammar is deliberately close to the expressions used by Obsidian Bases
//! (`price * quantity`, `if(done, "yes", "no")`, `file.name`), but evaluation
//! never touches the filesystem, the clock is only read by `now()`/`today()`,
//! and unknown fields evaluate to null instead of failing.
//!
//! Values are `null`, booleans, numbers, strings and lists. List fields such as
//! `tags` are real lists; `+` concatenates when either side is a string.
//!
//! The module is pure: parsing is separate from evaluation, so a caller can
//! validate an expression once and run it against many records.

use crate::record::Record;
use crate::storage::{host_of, record_filename};
use chrono::{Local, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Vec<Value>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Truthiness used by `if`, `&&`, `||` and `!`. Empty strings/lists and
    /// zero are false; every other value is true.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::List(items) => !items.is_empty(),
        }
    }

    /// Human-readable form used by `string()`, `+` with a string, and rule
    /// assignment. Lists join with `", "`.
    pub fn display(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Num(n) => number_display(*n),
            Value::Str(s) => s.clone(),
            Value::List(items) => items
                .iter()
                .map(|v| v.display())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    fn as_num(&self) -> Result<f64, String> {
        match self {
            Value::Null => Ok(0.0),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Num(n) => Ok(*n),
            Value::Str(s) => s
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("expected a number, got {:?}", s)),
            Value::List(_) => Err("expected a number, got a list".into()),
        }
    }

    fn as_str(&self) -> String {
        self.display()
    }
}

pub fn number_display(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    /// A dotted identifier path, e.g. `price`, `file.name`, `note.tags`.
    Field(Vec<String>),
    /// Property access on an arbitrary expression, e.g. `(a + b).length`.
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Method(Box<Expr>, String, Vec<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
}

/// Parse an expression once. A parse error is reported with enough context to
/// find the bad token; evaluation cannot fail on syntax after this point.
pub fn parse_formula(text: &str) -> Result<Expr, String> {
    let tokens = tokenize(text)?;
    let mut p = Parser { tokens, pos: 0 };
    let expr = p.parse_ternary()?;
    if !matches!(p.peek(), Tok::Eof) {
        return Err(format!("unexpected token {}", p.peek().describe()));
    }
    Ok(expr)
}

pub fn eval_formula(expr: &Expr, record: &Record) -> Result<Value, String> {
    eval(expr, record)
}

/// Parse and evaluate in one step. Useful for previews and tests; rules parse
/// once at load time instead.
pub fn evaluate(text: &str, record: &Record) -> Result<Value, String> {
    let expr = parse_formula(text)?;
    eval_formula(&expr, record)
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Question,
    Colon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Eof,
}

impl Tok {
    fn describe(&self) -> String {
        match self {
            Tok::Num(n) => format!("number {}", n),
            Tok::Str(s) => format!("string {:?}", s),
            Tok::Ident(s) => format!("`{}`", s),
            Tok::LParen => "'('".into(),
            Tok::RParen => "')'".into(),
            Tok::LBracket => "'['".into(),
            Tok::RBracket => "']'".into(),
            Tok::Comma => "','".into(),
            Tok::Dot => "'.'".into(),
            Tok::Question => "'?'".into(),
            Tok::Colon => "':'".into(),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Star => "'*'".into(),
            Tok::Slash => "'/'".into(),
            Tok::Percent => "'%'".into(),
            Tok::Bang => "'!'".into(),
            Tok::EqEq => "'=='".into(),
            Tok::NotEq => "'!='".into(),
            Tok::Lt => "'<'".into(),
            Tok::Le => "'<='".into(),
            Tok::Gt => "'>'".into(),
            Tok::Ge => "'>='".into(),
            Tok::AndAnd => "'&&'".into(),
            Tok::OrOr => "'||'".into(),
            Tok::Eof => "end of expression".into(),
        }
    }
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut buf = String::new();
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == quote {
                    closed = true;
                    i += 1;
                    break;
                }
                if ch == '\\' && i + 1 < chars.len() {
                    i += 1;
                    buf.push(match chars[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                    i += 1;
                    continue;
                }
                buf.push(ch);
                i += 1;
            }
            if !closed {
                return Err(format!("unterminated string at offset {}", i));
            }
            out.push(Tok::Str(buf));
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let raw: String = chars[start..i].iter().collect();
            let n = raw
                .parse::<f64>()
                .map_err(|_| format!("invalid number {:?}", raw))?;
            out.push(Tok::Num(n));
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect()));
            continue;
        }
        let two = |a: char, b: char| -> bool {
            i + 1 < chars.len() && chars[i] == a && chars[i + 1] == b
        };
        if two('=', '=') {
            out.push(Tok::EqEq);
            i += 2;
        } else if two('!', '=') {
            out.push(Tok::NotEq);
            i += 2;
        } else if two('<', '=') {
            out.push(Tok::Le);
            i += 2;
        } else if two('>', '=') {
            out.push(Tok::Ge);
            i += 2;
        } else if two('&', '&') {
            out.push(Tok::AndAnd);
            i += 2;
        } else if two('|', '|') {
            out.push(Tok::OrOr);
            i += 2;
        } else {
            let tok = match c {
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                '[' => Tok::LBracket,
                ']' => Tok::RBracket,
                ',' => Tok::Comma,
                '.' => Tok::Dot,
                '?' => Tok::Question,
                ':' => Tok::Colon,
                '+' => Tok::Plus,
                '-' => Tok::Minus,
                '*' => Tok::Star,
                '/' => Tok::Slash,
                '%' => Tok::Percent,
                '!' => Tok::Bang,
                '<' => Tok::Lt,
                '>' => Tok::Gt,
                other => return Err(format!("unexpected character {:?} at offset {}", other, i)),
            };
            out.push(tok);
            i += 1;
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        self.tokens.get(self.pos).unwrap_or(&Tok::Eof)
    }

    fn next(&mut self) -> Tok {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Tok::Eof);
        self.pos += 1;
        t
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == tok {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, tok: Tok) -> Result<(), String> {
        if self.eat(&tok) {
            Ok(())
        } else {
            Err(format!(
                "expected {}, found {}",
                tok.describe(),
                self.peek().describe()
            ))
        }
    }

    fn is_keyword(&self, word: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == word)
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_or()?;
        if self.eat(&Tok::Question) {
            let then = self.parse_ternary()?;
            self.expect(Tok::Colon)?;
            let other = self.parse_ternary()?;
            return Ok(Expr::Ternary(
                Box::new(cond),
                Box::new(then),
                Box::new(other),
            ));
        }
        Ok(cond)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        loop {
            if self.eat(&Tok::OrOr) || self.is_keyword("or") {
                if matches!(self.peek(), Tok::Ident(s) if s == "or") {
                    self.next();
                }
                let right = self.parse_and()?;
                left = Expr::Binary(BinOp::Or, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;
        loop {
            if self.eat(&Tok::AndAnd) || self.is_keyword("and") {
                if matches!(self.peek(), Tok::Ident(s) if s == "and") {
                    self.next();
                }
                let right = self.parse_not()?;
                left = Expr::Binary(BinOp::And, Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    /// `not` is deliberately low-precedence so `not status == "x"` means
    /// `not (status == "x")`, matching the rule engine. `!` stays tight.
    fn parse_not(&mut self) -> Result<Expr, String> {
        if self.is_keyword("not") {
            self.next();
            let inner = self.parse_not()?;
            return Ok(Expr::Unary(UnOp::Not, Box::new(inner)));
        }
        self.parse_equality()
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Tok::EqEq => Some(BinOp::Eq),
                Tok::NotEq => Some(BinOp::Ne),
                _ => None,
            };
            let Some(op) = op else { break };
            self.next();
            let right = self.parse_comparison()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Tok::Lt => Some(BinOp::Lt),
                Tok::Le => Some(BinOp::Le),
                Tok::Gt => Some(BinOp::Gt),
                Tok::Ge => Some(BinOp::Ge),
                _ => None,
            };
            let Some(op) = op else { break };
            self.next();
            let right = self.parse_additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => Some(BinOp::Add),
                Tok::Minus => Some(BinOp::Sub),
                _ => None,
            };
            let Some(op) = op else { break };
            self.next();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => Some(BinOp::Mul),
                Tok::Slash => Some(BinOp::Div),
                Tok::Percent => Some(BinOp::Mod),
                _ => None,
            };
            let Some(op) = op else { break };
            self.next();
            let right = self.parse_unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Tok::Bang => {
                self.next();
                let inner = self.parse_unary()?;
                Ok(Expr::Unary(UnOp::Not, Box::new(inner)))
            }
            Tok::Minus => {
                self.next();
                let inner = self.parse_unary()?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(inner)))
            }
            Tok::Plus => {
                self.next();
                self.parse_unary()
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if matches!(self.peek(), Tok::LParen)
                && matches!(expr, Expr::Field(ref p) if p.len() == 1)
            {
                let name = match &expr {
                    Expr::Field(parts) => parts[0].clone(),
                    _ => unreachable!(),
                };
                self.next();
                let args = self.parse_args()?;
                expr = Expr::Call(name, args);
                continue;
            }
            if self.eat(&Tok::Dot) {
                let name = match self.next() {
                    Tok::Ident(s) => s,
                    other => {
                        return Err(format!(
                            "expected a property name after '.', found {}",
                            other.describe()
                        ));
                    }
                };
                if self.eat(&Tok::LParen) {
                    let args = self.parse_args()?;
                    expr = Expr::Method(Box::new(expr), name, args);
                } else {
                    expr = match expr {
                        Expr::Field(mut parts) => {
                            parts.push(name);
                            Expr::Field(parts)
                        }
                        other => Expr::Member(Box::new(other), name),
                    };
                }
                continue;
            }
            if self.eat(&Tok::LBracket) {
                let index = self.parse_ternary()?;
                self.expect(Tok::RBracket)?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if self.eat(&Tok::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_ternary()?);
            if self.eat(&Tok::Comma) {
                continue;
            }
            self.expect(Tok::RParen)?;
            return Ok(args);
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Tok::Num(n) => Ok(Expr::Literal(Value::Num(n))),
            Tok::Str(s) => Ok(Expr::Literal(Value::Str(s))),
            Tok::Ident(s) => match s.as_str() {
                "true" => Ok(Expr::Literal(Value::Bool(true))),
                "false" => Ok(Expr::Literal(Value::Bool(false))),
                "null" | "nil" => Ok(Expr::Literal(Value::Null)),
                _ => Ok(Expr::Field(vec![s])),
            },
            Tok::LParen => {
                let expr = self.parse_ternary()?;
                self.expect(Tok::RParen)?;
                Ok(expr)
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if self.eat(&Tok::RBracket) {
                    return Ok(Expr::List(items));
                }
                loop {
                    items.push(self.parse_ternary()?);
                    if self.eat(&Tok::Comma) {
                        continue;
                    }
                    self.expect(Tok::RBracket)?;
                    return Ok(Expr::List(items));
                }
            }
            other => Err(format!("expected a value, found {}", other.describe())),
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

fn eval(expr: &Expr, record: &Record) -> Result<Value, String> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Field(parts) => Ok(resolve_path(record, parts)),
        Expr::Member(base, name) => {
            let base = eval(base, record)?;
            Ok(member(&base, name))
        }
        Expr::Index(base, index) => {
            let base = eval(base, record)?;
            let index = eval(index, record)?;
            Ok(index_value(&base, &index))
        }
        Expr::Call(name, args) => call(name, args, record),
        Expr::Method(base, name, args) => {
            let base = eval(base, record)?;
            let mut values = Vec::with_capacity(args.len());
            for a in args {
                values.push(eval(a, record)?);
            }
            method(&base, name, &values)
        }
        Expr::Unary(op, inner) => {
            let v = eval(inner, record)?;
            Ok(match op {
                UnOp::Not => Value::Bool(!v.truthy()),
                UnOp::Neg => Value::Num(-v.as_num()?),
            })
        }
        Expr::Binary(op, left, right) => {
            if *op == BinOp::And {
                let l = eval(left, record)?;
                if !l.truthy() {
                    return Ok(Value::Bool(false));
                }
                return Ok(Value::Bool(eval(right, record)?.truthy()));
            }
            if *op == BinOp::Or {
                let l = eval(left, record)?;
                if l.truthy() {
                    return Ok(Value::Bool(true));
                }
                return Ok(Value::Bool(eval(right, record)?.truthy()));
            }
            let l = eval(left, record)?;
            let r = eval(right, record)?;
            binary(*op, l, r)
        }
        Expr::Ternary(cond, then, other) => {
            if eval(cond, record)?.truthy() {
                eval(then, record)
            } else {
                eval(other, record)
            }
        }
        Expr::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(eval(item, record)?);
            }
            Ok(Value::List(out))
        }
    }
}

fn resolve_path(record: &Record, parts: &[String]) -> Value {
    let Some((root, rest)) = parts.split_first() else {
        return Value::Null;
    };
    match root.as_str() {
        "file" => file_path(record, rest),
        "note" | "record" | "property" => {
            let mut value = rest
                .first()
                .map(|name| metadata_field(record, name))
                .unwrap_or(Value::Null);
            for name in rest.iter().skip(1) {
                value = member(&value, name);
            }
            value
        }
        _ => {
            let mut value = metadata_field(record, root);
            for name in rest {
                value = member(&value, name);
            }
            value
        }
    }
}

fn file_path(record: &Record, rest: &[String]) -> Value {
    let Some(name) = rest.first() else {
        return Value::Null;
    };
    let value = match name.as_str() {
        "name" => Value::Str(
            record
                .title
                .clone()
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| record.id.clone()),
        ),
        "path" => {
            let folder = record
                .folder
                .clone()
                .unwrap_or_else(|| record.r#type.clone());
            Value::Str(format!("{}/{}", folder, record_filename(record)))
        }
        "folder" => Value::Str(
            record
                .folder
                .clone()
                .unwrap_or_else(|| record.r#type.clone()),
        ),
        "ext" | "extension" => Value::Str("md".into()),
        "tags" => Value::List(record.tags.iter().cloned().map(Value::Str).collect()),
        "links" => Value::List(record.links.iter().cloned().map(Value::Str).collect()),
        "ctime" | "created" => record
            .created
            .clone()
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "mtime" | "modified" => record
            .modified
            .clone()
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "size" => Value::Null,
        _ => Value::Null,
    };
    rest.iter().skip(1).fold(value, |v, p| member(&v, p))
}

fn metadata_field(record: &Record, name: &str) -> Value {
    match name {
        "id" => Value::Str(record.id.clone()),
        "type" => Value::Str(record.r#type.clone()),
        "target_type" => record
            .target_type
            .clone()
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "title" => record.title.clone().map(Value::Str).unwrap_or(Value::Null),
        "icon" => record.icon.clone().map(Value::Str).unwrap_or(Value::Null),
        "color" => record.color.clone().map(Value::Str).unwrap_or(Value::Null),
        "description" => record
            .description
            .clone()
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "status" => record.status.clone().map(Value::Str).unwrap_or(Value::Null),
        "url" => record.url.clone().map(Value::Str).unwrap_or(Value::Null),
        "folder" => Value::Str(record.folder.clone().unwrap_or_default()),
        "created" => record
            .created
            .clone()
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "modified" => record
            .modified
            .clone()
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "tags" => Value::List(record.tags.iter().cloned().map(Value::Str).collect()),
        "categories" => Value::List(record.categories.iter().cloned().map(Value::Str).collect()),
        "related" => Value::List(record.related.iter().cloned().map(Value::Str).collect()),
        "links" => Value::List(record.links.iter().cloned().map(Value::Str).collect()),
        "parent" => record.parent.clone().map(Value::Str).unwrap_or(Value::Null),
        "host" | "domain" | "site" => record
            .url
            .as_deref()
            .and_then(host_of)
            .map(|h| Value::Str(crate::naming::normalize_host(h)))
            .unwrap_or(Value::Null),
        other => record
            .extra
            .get(other)
            .map(yaml_to_value)
            .unwrap_or(Value::Null),
    }
}

fn yaml_to_value(value: &serde_yaml::Value) -> Value {
    match value {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => n.as_f64().map(Value::Num).unwrap_or(Value::Null),
        serde_yaml::Value::String(s) => Value::Str(s.clone()),
        serde_yaml::Value::Sequence(items) => {
            Value::List(items.iter().map(yaml_to_value).collect())
        }
        _ => Value::Null,
    }
}

fn member(value: &Value, name: &str) -> Value {
    match name {
        "length" | "size" => match value {
            Value::Str(s) => Value::Num(s.chars().count() as f64),
            Value::List(items) => Value::Num(items.len() as f64),
            Value::Null => Value::Num(0.0),
            other => Value::Num(other.display().chars().count() as f64),
        },
        "first" => match value {
            Value::List(items) => items.first().cloned().unwrap_or(Value::Null),
            Value::Str(s) => s
                .chars()
                .next()
                .map(|c| Value::Str(c.to_string()))
                .unwrap_or(Value::Null),
            _ => Value::Null,
        },
        "last" => match value {
            Value::List(items) => items.last().cloned().unwrap_or(Value::Null),
            Value::Str(s) => s
                .chars()
                .last()
                .map(|c| Value::Str(c.to_string()))
                .unwrap_or(Value::Null),
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}

fn index_value(base: &Value, index: &Value) -> Value {
    let Ok(raw) = index.as_num() else {
        return Value::Null;
    };
    let idx = raw as isize;
    match base {
        Value::List(items) => {
            let len = items.len() as isize;
            let real = if idx < 0 { len + idx } else { idx };
            if real < 0 || real >= len {
                Value::Null
            } else {
                items[real as usize].clone()
            }
        }
        Value::Str(s) => {
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as isize;
            let real = if idx < 0 { len + idx } else { idx };
            if real < 0 || real >= len {
                Value::Null
            } else {
                Value::Str(chars[real as usize].to_string())
            }
        }
        _ => Value::Null,
    }
}

fn binary(op: BinOp, left: Value, right: Value) -> Result<Value, String> {
    match op {
        BinOp::Add => {
            if matches!(left, Value::Str(_)) || matches!(right, Value::Str(_)) {
                Ok(Value::Str(format!("{}{}", left.as_str(), right.as_str())))
            } else if matches!((&left, &right), (Value::List(_), Value::List(_))) {
                let (Value::List(mut a), Value::List(b)) = (left, right) else {
                    unreachable!("checked above");
                };
                a.extend(b);
                Ok(Value::List(a))
            } else {
                Ok(Value::Num(left.as_num()? + right.as_num()?))
            }
        }
        BinOp::Sub => Ok(Value::Num(left.as_num()? - right.as_num()?)),
        BinOp::Mul => Ok(Value::Num(left.as_num()? * right.as_num()?)),
        BinOp::Div => {
            let d = right.as_num()?;
            if d == 0.0 {
                return Err("division by zero".into());
            }
            Ok(Value::Num(left.as_num()? / d))
        }
        BinOp::Mod => {
            let d = right.as_num()?;
            if d == 0.0 {
                return Err("division by zero".into());
            }
            Ok(Value::Num(left.as_num()? % d))
        }
        BinOp::Eq => Ok(Value::Bool(values_equal(&left, &right))),
        BinOp::Ne => Ok(Value::Bool(!values_equal(&left, &right))),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => compare(op, &left, &right),
        BinOp::And | BinOp::Or => unreachable!("short-circuited above"),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Num(x), Value::Num(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| values_equal(a, b))
        }
        _ => false,
    }
}

fn compare(op: BinOp, left: &Value, right: &Value) -> Result<Value, String> {
    let result = match (left, right) {
        (Value::Str(a), Value::Str(b)) => match op {
            BinOp::Lt => a < b,
            BinOp::Le => a <= b,
            BinOp::Gt => a > b,
            BinOp::Ge => a >= b,
            _ => unreachable!(),
        },
        _ => {
            let a = left.as_num()?;
            let b = right.as_num()?;
            match op {
                BinOp::Lt => a < b,
                BinOp::Le => a <= b,
                BinOp::Gt => a > b,
                BinOp::Ge => a >= b,
                _ => unreachable!(),
            }
        }
    };
    Ok(Value::Bool(result))
}

fn call(name: &str, args: &[Expr], record: &Record) -> Result<Value, String> {
    match name {
        // Lazy by design: the unused branch of `if` must not fail (or be
        // evaluated at all), which is what makes guard patterns work.
        "if" => {
            if args.len() != 3 {
                return Err("if() takes three arguments".into());
            }
            if eval(&args[0], record)?.truthy() {
                eval(&args[1], record)
            } else {
                eval(&args[2], record)
            }
        }
        "coalesce" | "default" => {
            if args.is_empty() {
                return Err(format!("{}() needs at least one argument", name));
            }
            for arg in args {
                let value = eval(arg, record)?;
                if !value.is_null() {
                    return Ok(value);
                }
            }
            Ok(Value::Null)
        }
        "now" => {
            expect_arity(name, args, 0)?;
            Ok(Value::Str(Utc::now().to_rfc3339()))
        }
        "today" => {
            expect_arity(name, args, 0)?;
            Ok(Value::Str(Local::now().format("%Y-%m-%d").to_string()))
        }
        _ => {
            let mut values = Vec::with_capacity(args.len());
            for arg in args {
                values.push(eval(arg, record)?);
            }
            eager_call(name, &values)
        }
    }
}

fn expect_arity(name: &str, args: &[Expr], want: usize) -> Result<(), String> {
    if args.len() == want {
        Ok(())
    } else {
        Err(format!("{}() takes {} argument(s)", name, want))
    }
}

fn eager_call(name: &str, args: &[Value]) -> Result<Value, String> {
    match name {
        "min" | "max" => {
            if args.is_empty() {
                return Ok(Value::Null);
            }
            let mut nums = Vec::new();
            for arg in args {
                match arg {
                    Value::List(items) => {
                        for item in items {
                            nums.push(item.as_num()?);
                        }
                    }
                    other => nums.push(other.as_num()?),
                }
            }
            let value = if name == "min" {
                nums.into_iter().fold(f64::INFINITY, f64::min)
            } else {
                nums.into_iter().fold(f64::NEG_INFINITY, f64::max)
            };
            Ok(Value::Num(value))
        }
        "sum" | "average" => {
            let mut nums = Vec::new();
            for arg in args {
                match arg {
                    Value::List(items) => {
                        for item in items {
                            nums.push(item.as_num()?);
                        }
                    }
                    other => nums.push(other.as_num()?),
                }
            }
            if nums.is_empty() {
                return Ok(Value::Num(0.0));
            }
            let sum: f64 = nums.iter().sum();
            let value = if name == "average" {
                sum / nums.len() as f64
            } else {
                sum
            };
            Ok(Value::Num(value))
        }
        "abs" => one_num(name, args).map(|n| Value::Num(n.abs())),
        "floor" => one_num(name, args).map(|n| Value::Num(n.floor())),
        "ceil" => one_num(name, args).map(|n| Value::Num(n.ceil())),
        "sqrt" => one_num(name, args).map(|n| Value::Num(n.sqrt())),
        "round" => {
            if args.len() == 1 {
                Ok(Value::Num(args[0].as_num()?.round()))
            } else if args.len() == 2 {
                let digits = args[1].as_num()?.max(0.0) as i32;
                let factor = 10f64.powi(digits);
                Ok(Value::Num((args[0].as_num()? * factor).round() / factor))
            } else {
                Err("round() takes one or two arguments".into())
            }
        }
        "length" => {
            if args.len() != 1 {
                return Err("length() takes one argument".into());
            }
            Ok(member(&args[0], "length"))
        }
        "lower" | "lowercase" => one_str(name, args).map(|s| Value::Str(s.to_lowercase())),
        "upper" | "uppercase" => one_str(name, args).map(|s| Value::Str(s.to_uppercase())),
        "trim" => one_str(name, args).map(|s| Value::Str(s.trim().to_string())),
        "string" => {
            if args.len() != 1 {
                return Err("string() takes one argument".into());
            }
            Ok(Value::Str(args[0].display()))
        }
        "number" => {
            if args.len() != 1 {
                return Err("number() takes one argument".into());
            }
            Ok(Value::Num(args[0].as_num()?))
        }
        "boolean" | "bool" => {
            if args.len() != 1 {
                return Err("boolean() takes one argument".into());
            }
            Ok(Value::Bool(args[0].truthy()))
        }
        "list" => {
            if args.len() == 1 && matches!(args[0], Value::List(_)) {
                return Ok(args[0].clone());
            }
            Ok(Value::List(args.to_vec()))
        }
        "contains" => {
            if args.len() != 2 {
                return Err("contains() takes two arguments".into());
            }
            Ok(Value::Bool(contains(&args[0], &args[1])))
        }
        "startsWith" | "startswith" => {
            if args.len() != 2 {
                return Err("startsWith() takes two arguments".into());
            }
            Ok(Value::Bool(args[0].as_str().starts_with(&args[1].as_str())))
        }
        "endsWith" | "endswith" => {
            if args.len() != 2 {
                return Err("endsWith() takes two arguments".into());
            }
            Ok(Value::Bool(args[0].as_str().ends_with(&args[1].as_str())))
        }
        "replace" => {
            if args.len() != 3 {
                return Err("replace() takes three arguments".into());
            }
            Ok(Value::Str(
                args[0]
                    .as_str()
                    .replace(&args[1].as_str(), &args[2].as_str()),
            ))
        }
        "join" => {
            if args.len() != 2 {
                return Err("join() takes two arguments".into());
            }
            match &args[0] {
                Value::List(items) => Ok(Value::Str(
                    items
                        .iter()
                        .map(|v| v.display())
                        .collect::<Vec<_>>()
                        .join(&args[1].as_str()),
                )),
                other => Ok(Value::Str(other.display())),
            }
        }
        "pow" => {
            if args.len() != 2 {
                return Err("pow() takes two arguments".into());
            }
            Ok(Value::Num(args[0].as_num()?.powf(args[1].as_num()?)))
        }
        _ => Err(format!("unknown function {}()", name)),
    }
}

fn one_num(name: &str, args: &[Value]) -> Result<f64, String> {
    if args.len() != 1 {
        return Err(format!("{}() takes one argument", name));
    }
    args[0].as_num()
}

fn one_str(name: &str, args: &[Value]) -> Result<String, String> {
    if args.len() != 1 {
        return Err(format!("{}() takes one argument", name));
    }
    Ok(args[0].as_str())
}

fn contains(haystack: &Value, needle: &Value) -> bool {
    match haystack {
        Value::List(items) => items.iter().any(|v| values_equal(v, needle)),
        Value::Str(s) => s.contains(&needle.as_str()),
        Value::Null => false,
        other => other.display().contains(&needle.as_str()),
    }
}

fn method(base: &Value, name: &str, args: &[Value]) -> Result<Value, String> {
    match name {
        "toUpperCase" | "toUpperCase" if args.is_empty() => {
            Ok(Value::Str(base.as_str().to_uppercase()))
        }
        "toLowerCase" if args.is_empty() => Ok(Value::Str(base.as_str().to_lowercase())),
        "trim" if args.is_empty() => Ok(Value::Str(base.as_str().trim().to_string())),
        "toFixed" => {
            if args.len() != 1 {
                return Err("toFixed() takes one argument".into());
            }
            let digits = args[0].as_num()?.max(0.0) as usize;
            Ok(Value::Str(format!("{:.*}", digits, base.as_num()?)))
        }
        "includes" | "contains" => {
            if args.len() != 1 {
                return Err(format!("{}() takes one argument", name));
            }
            Ok(Value::Bool(contains(base, &args[0])))
        }
        "startsWith" => {
            if args.len() != 1 {
                return Err("startsWith() takes one argument".into());
            }
            Ok(Value::Bool(base.as_str().starts_with(&args[0].as_str())))
        }
        "endsWith" => {
            if args.len() != 1 {
                return Err("endsWith() takes one argument".into());
            }
            Ok(Value::Bool(base.as_str().ends_with(&args[0].as_str())))
        }
        "replace" => {
            if args.len() != 2 {
                return Err("replace() takes two arguments".into());
            }
            Ok(Value::Str(
                base.as_str().replace(&args[0].as_str(), &args[1].as_str()),
            ))
        }
        "split" => {
            if args.len() != 1 {
                return Err("split() takes one argument".into());
            }
            let sep = args[0].as_str();
            let parts: Vec<Value> = if sep.is_empty() {
                base.as_str()
                    .chars()
                    .map(|c| Value::Str(c.to_string()))
                    .collect()
            } else {
                base.as_str()
                    .split(&sep)
                    .map(|s| Value::Str(s.to_string()))
                    .collect()
            };
            Ok(Value::List(parts))
        }
        "join" => {
            if args.len() != 1 {
                return Err("join() takes one argument".into());
            }
            match base {
                Value::List(items) => Ok(Value::Str(
                    items
                        .iter()
                        .map(|v| v.display())
                        .collect::<Vec<_>>()
                        .join(&args[0].as_str()),
                )),
                other => Ok(Value::Str(other.display())),
            }
        }
        "slice" => {
            if args.is_empty() || args.len() > 2 {
                return Err("slice() takes one or two arguments".into());
            }
            let start = args[0].as_num()? as isize;
            match base {
                Value::Str(s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let len = chars.len() as isize;
                    let start = if start < 0 { len + start } else { start };
                    let end = if args.len() == 2 {
                        let e = args[1].as_num()? as isize;
                        if e < 0 { len + e } else { e }
                    } else {
                        len
                    };
                    let start = start.clamp(0, len) as usize;
                    let end = end.clamp(0, len) as usize;
                    Ok(Value::Str(chars[start..end.max(start)].iter().collect()))
                }
                Value::List(items) => {
                    let len = items.len() as isize;
                    let start = if start < 0 { len + start } else { start };
                    let end = if args.len() == 2 {
                        let e = args[1].as_num()? as isize;
                        if e < 0 { len + e } else { e }
                    } else {
                        len
                    };
                    let start = start.clamp(0, len) as usize;
                    let end = end.clamp(0, len) as usize;
                    Ok(Value::List(items[start..end.max(start)].to_vec()))
                }
                _ => Ok(Value::Null),
            }
        }
        "at" => {
            if args.len() != 1 {
                return Err("at() takes one argument".into());
            }
            Ok(index_value(base, &args[0]))
        }
        "isEmpty" | "isempty" => {
            if !args.is_empty() {
                return Err("isEmpty() takes no arguments".into());
            }
            Ok(Value::Bool(match base {
                Value::Null => true,
                Value::List(items) => items.is_empty(),
                Value::Str(s) => s.is_empty(),
                _ => false,
            }))
        }
        _ => Err(format!("unknown method .{}()", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value as Yaml;

    fn record() -> Record {
        let mut r = Record::new("rec_1", "note");
        r.title = Some("Hello World".to_string());
        r.status = Some("Reading".to_string());
        r.folder = Some("Inbox".to_string());
        r.url = Some("https://www.example.com/post".to_string());
        r.tags = vec!["rust".to_string(), "cli".to_string()];
        r.extra
            .insert("price".into(), Yaml::Number(serde_yaml::Number::from(12.5)));
        r.extra
            .insert("quantity".into(), Yaml::Number(serde_yaml::Number::from(3)));
        r.extra.insert("done".into(), Yaml::Bool(false));
        r
    }

    fn eval_str(text: &str) -> Result<Value, String> {
        evaluate(text, &record())
    }

    #[test]
    fn arithmetic_precedence_and_parens() {
        assert_eq!(eval_str("2 + 3 * 4").unwrap(), Value::Num(14.0));
        assert_eq!(eval_str("(2 + 3) * 4").unwrap(), Value::Num(20.0));
        assert_eq!(eval_str("10 % 4").unwrap(), Value::Num(2.0));
        assert_eq!(eval_str("-2 * -3").unwrap(), Value::Num(6.0));
    }

    #[test]
    fn strings_and_concat() {
        assert_eq!(
            eval_str(r#""a" + 2 + "b""#).unwrap(),
            Value::Str("a2b".into())
        );
        assert_eq!(eval_str(r#""10" * 2"#).unwrap(), Value::Num(20.0));
        assert_eq!(
            eval_str(r#"title.toUpperCase()"#).unwrap(),
            Value::Str("HELLO WORLD".into())
        );
        assert_eq!(
            eval_str(r#"title.replace("World", "Rust")"#).unwrap(),
            Value::Str("Hello Rust".into())
        );
    }

    #[test]
    fn fields_and_nested_metadata() {
        assert_eq!(eval_str("price * quantity").unwrap(), Value::Num(37.5));
        assert_eq!(
            eval_str("file.name").unwrap(),
            Value::Str("Hello World".into())
        );
        assert_eq!(eval_str("file.folder").unwrap(), Value::Str("Inbox".into()));
        assert_eq!(eval_str("host").unwrap(), Value::Str("example.com".into()));
        assert_eq!(
            eval_str("status == \"Reading\"").unwrap(),
            Value::Bool(true)
        );
        assert!(eval_str("missing").unwrap().is_null());
        assert!(eval_str("missing.deeper").unwrap().is_null());
    }

    #[test]
    fn lists_index_and_methods() {
        assert_eq!(eval_str("tags.length").unwrap(), Value::Num(2.0));
        assert_eq!(eval_str("tags[0]").unwrap(), Value::Str("rust".into()));
        assert_eq!(eval_str("tags[-1]").unwrap(), Value::Str("cli".into()));
        assert_eq!(
            eval_str("tags.join(\"/\")").unwrap(),
            Value::Str("rust/cli".into())
        );
        assert_eq!(
            eval_str("tags.contains(\"rust\")").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            eval_str(r#""a,b,c".split(",")[1]"#).unwrap(),
            Value::Str("b".into())
        );
    }

    #[test]
    fn conditions_ternary_and_lazy_if() {
        assert_eq!(
            eval_str(r#"price > 10 && status == "Reading""#).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            eval_str(r#"not status == "Archived""#).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            eval_str(r#"done ? "yes" : "no""#).unwrap(),
            Value::Str("no".into())
        );
        // The unused branch is never evaluated, so an unknown function there
        // does not make the whole expression fail.
        assert_eq!(
            eval_str(r#"if(true, 1, unknownFn())"#).unwrap(),
            Value::Num(1.0)
        );
        assert_eq!(
            eval_str(r#"coalesce(missing, "fallback")"#).unwrap(),
            Value::Str("fallback".into())
        );
    }

    #[test]
    fn numeric_functions() {
        assert_eq!(eval_str("min(3, 1, 2)").unwrap(), Value::Num(1.0));
        assert_eq!(eval_str("max([3, 9, 2])").unwrap(), Value::Num(9.0));
        assert_eq!(eval_str("sum(tags.length, 3)").unwrap(), Value::Num(5.0));
        assert_eq!(eval_str("round(12.345, 2)").unwrap(), Value::Num(12.35));
        assert_eq!(
            eval_str("price.toFixed(2)").unwrap(),
            Value::Str("12.50".into())
        );
        assert_eq!(eval_str("abs(-4)").unwrap(), Value::Num(4.0));
    }

    #[test]
    fn dates_have_expected_shape() {
        let today = eval_str("today()").unwrap().display();
        assert_eq!(today.len(), 10);
        let now = eval_str("now()").unwrap().display();
        assert!(now.contains('T'));
    }

    #[test]
    fn division_by_zero_is_an_error() {
        assert!(eval_str("1 / 0").is_err());
        assert!(eval_str("1 % 0").is_err());
    }

    #[test]
    fn syntax_errors_are_reported() {
        for bad in ["1 +", "(1 + 2", "1 ? 2", r#""unclosed"#, "1 @ 2", "a."] {
            assert!(parse_formula(bad).is_err(), "should fail: {bad}");
        }
    }

    #[test]
    fn unknown_function_is_an_error() {
        assert!(eval_str("unknownFn()").is_err());
        assert!(eval_str(r#""x".unknownMethod()"#).is_err());
    }

    #[test]
    fn equality_is_strict_about_types() {
        assert_eq!(eval_str("1 == 1").unwrap(), Value::Bool(true));
        assert_eq!(eval_str(r#""1" == 1"#).unwrap(), Value::Bool(false));
        assert_eq!(
            eval_str(r#"tags == ["rust", "cli"]"#).unwrap(),
            Value::Bool(true)
        );
    }
}
