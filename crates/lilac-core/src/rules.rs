//! Minimal rule engine for Lilac records.
//!
//! A rule is: `when <condition> then <action>` (possibly multiple `then`).
//! Conditions combine with `and` / `or` / `not` and atoms:
//!     - `true` / `false`
//!     - `tag "x"` (record has tag x)
//!     - `field == "value"` / `field != "value"`
//!     - `field contains "value"` / `field has "value"`
//!     - parenthesized groups
//! Actions:
//!     - `set field = value`
//!     - `add tag "x"`
//!     - `remove tag "x"`
//!     - `move "folder"`
//!     - `delete`
//!
//! Syntax is intentionally minimal and subject to change; the engine is the
//! stable part.

use crate::record::Record;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    True,
    False,
    Eq(String, Value),
    Ne(String, Value),
    Contains(String, String),
    HasTag(String),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Set(String, Value),
    AddTag(String),
    RemoveTag(String),
    Move(String),
    Delete,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: Option<String>,
    pub when: Condition,
    pub then: Vec<Action>,
}

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

/// Apply rules to a record in order. Returns the actions that fired.
pub fn apply_rules(record: &mut Record, rules: &[Rule]) -> Vec<Action> {
    let mut fired = Vec::new();
    for rule in rules {
        if eval_condition(&rule.when, record) {
            for a in &rule.then {
                apply_action(record, a);
                fired.push(a.clone());
            }
        }
    }
    fired
}

fn apply_action(record: &mut Record, a: &Action) {
    match a {
        Action::Set(field, val) => {
            set_field(record, field, val.clone());
        }
        Action::AddTag(t) => {
            if !record.tags.iter().any(|x| x == t) {
                record.tags.push(t.clone());
            }
        }
        Action::RemoveTag(t) => {
            record.tags.retain(|x| x != t);
        }
        Action::Move(folder) => {
            record.folder = Some(folder.clone());
        }
        Action::Delete => {
            record
                .extra
                .insert("_deleted".to_string(), serde_yaml::Value::Bool(true));
        }
    }
}

fn set_field(record: &mut Record, field: &str, val: Value) {
    match field {
        "title" => {
            record.title = Some(match val {
                Value::Str(s) => s,
                Value::Num(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
            })
        }
        "status" => {
            record.status = Some(match val {
                Value::Str(s) => s,
                Value::Num(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
            })
        }
        "url" => {
            record.url = Some(match val {
                Value::Str(s) => s,
                Value::Num(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
            })
        }
        "description" => {
            record.description = Some(match val {
                Value::Str(s) => s,
                Value::Num(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
            })
        }
        other => {
            let yv = match val {
                Value::Str(s) => serde_yaml::Value::String(s),
                Value::Num(n) => serde_yaml::Value::Number(serde_yaml::Number::from(n as i64)),
                Value::Bool(b) => serde_yaml::Value::Bool(b),
            };
            record.extra.insert(other.to_string(), yv);
        }
    }
}

fn get_field(record: &Record, field: &str) -> Option<Value> {
    match field {
        "title" => record.title.clone().map(Value::Str),
        "status" => record.status.clone().map(Value::Str),
        "url" => record.url.clone().map(Value::Str),
        "description" => record.description.clone().map(Value::Str),
        "type" => Some(Value::Str(record.r#type.clone())),
        "id" => Some(Value::Str(record.id.clone())),
        other => record.extra.get(other).map(yaml_to_value),
    }
}

fn yaml_to_value(v: &serde_yaml::Value) -> Value {
    match v {
        serde_yaml::Value::String(s) => Value::Str(s.clone()),
        serde_yaml::Value::Number(n) => Value::Num(n.as_f64().unwrap_or(0.0)),
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        _ => Value::Str(format!("{:?}", v)),
    }
}

fn eval_condition(c: &Condition, record: &Record) -> bool {
    match c {
        Condition::True => true,
        Condition::False => false,
        Condition::Eq(f, v) => get_field(record, f)
            .map(|gv| values_equal(&gv, v))
            .unwrap_or(false),
        Condition::Ne(f, v) => !eval_condition(&Condition::Eq(f.clone(), v.clone()), record),
        Condition::Contains(f, needle) => get_field(record, f)
            .map(|gv| match gv {
                Value::Str(s) => s.contains(needle),
                _ => false,
            })
            .unwrap_or(false),
        Condition::HasTag(t) => record.tags.iter().any(|x| x == t),
        Condition::And(a, b) => eval_condition(a, record) && eval_condition(b, record),
        Condition::Or(a, b) => eval_condition(a, record) || eval_condition(b, record),
        Condition::Not(x) => !eval_condition(x, record),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Num(x), Value::Num(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Num(_)) => x
            .parse::<f64>()
            .ok()
            .map(|n| {
                (n - match b {
                    Value::Num(y) => *y,
                    _ => 0.0,
                })
                .abs()
                    < f64::EPSILON
            })
            .unwrap_or(false),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

pub fn parse_rules(text: &str) -> Result<RuleSet, String> {
    let mut rules = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_when: Option<String> = None;
    let mut current_thens: Vec<String> = Vec::new();

    let flush = |name: &Option<String>,
                 w: &Option<String>,
                 thens: &Vec<String>,
                 rules: &mut Vec<Rule>|
     -> Result<(), String> {
        if w.is_none() && thens.is_empty() {
            return Ok(());
        }
        let w = w.as_ref().ok_or("rule missing 'when'")?;
        let cond = parse_condition(w)?;
        let mut actions = Vec::new();
        for t in thens {
            actions.push(parse_action(t)?);
        }
        if actions.is_empty() {
            return Err("rule missing 'then'".into());
        }
        rules.push(Rule {
            name: name.clone(),
            when: cond,
            then: actions,
        });
        Ok(())
    };

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            flush(&current_name, &current_when, &current_thens, &mut rules)?;
            current_name = None;
            current_when = None;
            current_thens.clear();
            continue;
        }
        if let Some(rest) = line.strip_prefix("rule ") {
            flush(&current_name, &current_when, &current_thens, &mut rules)?;
            current_when = None;
            current_thens.clear();
            let rest = rest.trim();
            let name = if let Some(stripped) = rest.strip_prefix('"') {
                // parse quoted name
                let end = stripped.find('"').ok_or("unterminated rule name")?;
                stripped[..end].to_string()
            } else {
                rest.to_string()
            };
            current_name = Some(name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("when") {
            let rest = rest.strip_prefix(':').unwrap_or(rest).trim();
            current_when = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("then") {
            let rest = rest.strip_prefix(':').unwrap_or(rest).trim();
            current_thens.push(rest.to_string());
            continue;
        }
        return Err(format!("unexpected line: {}", line));
    }
    flush(&current_name, &current_when, &current_thens, &mut rules)?;
    Ok(RuleSet { rules })
}

// ---------------------------------------------------------------------------
// Condition parser (recursive descent)
// ---------------------------------------------------------------------------

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    _phantom: std::marker::PhantomData<&'a ()>,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Str(String),
    Num(f64),
    LParen,
    RParen,
    Eq,
    Ne,
    Eof,
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '(' {
            chars.next();
            out.push(Token::LParen);
            continue;
        }
        if c == ')' {
            chars.next();
            out.push(Token::RParen);
            continue;
        }
        if c == '"' {
            chars.next();
            let mut buf = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '"' {
                    chars.next();
                    break;
                }
                buf.push(ch);
                chars.next();
            }
            out.push(Token::Str(buf));
            continue;
        }
        if c == '=' && chars.clone().nth(1) == Some('=') {
            chars.next();
            chars.next();
            out.push(Token::Eq);
            continue;
        }
        if c == '!' && chars.clone().nth(1) == Some('=') {
            chars.next();
            chars.next();
            out.push(Token::Ne);
            continue;
        }
        if c.is_alphanumeric() || c == '_' || c == '.' {
            let mut buf = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                    buf.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            // number?
            if let Ok(n) = buf.parse::<f64>() {
                out.push(Token::Num(n));
            } else {
                out.push(Token::Ident(buf));
            }
            continue;
        }
        return Err(format!("unexpected char: {}", c));
    }
    out.push(Token::Eof);
    Ok(out)
}

impl<'a> Parser<'a> {
    fn new(s: &str) -> Result<Self, String> {
        let tokens = tokenize(s)?;
        Ok(Self {
            tokens,
            pos: 0,
            _phantom: std::marker::PhantomData,
        })
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn next(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        t
    }

    fn parse_expr(&mut self) -> Result<Condition, String> {
        let mut left = self.parse_and()?;
        while let Token::Ident(k) = self.peek() {
            if k == "or" {
                self.next();
                let right = self.parse_and()?;
                left = Condition::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Condition, String> {
        let mut left = self.parse_not()?;
        while let Token::Ident(k) = self.peek() {
            if k == "and" {
                self.next();
                let right = self.parse_not()?;
                left = Condition::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Condition, String> {
        if let Token::Ident(k) = self.peek()
            && k == "not"
        {
            self.next();
            let inner = self.parse_not()?;
            return Ok(Condition::Not(Box::new(inner)));
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<Condition, String> {
        match self.peek().clone() {
            Token::LParen => {
                self.next();
                let c = self.parse_expr()?;
                if !matches!(self.next(), Token::RParen) {
                    return Err("expected )".into());
                }
                Ok(c)
            }
            Token::Ident(k) if k == "true" => {
                self.next();
                Ok(Condition::True)
            }
            Token::Ident(k) if k == "false" => {
                self.next();
                Ok(Condition::False)
            }
            Token::Ident(k) if k == "tag" => {
                self.next();
                match self.next() {
                    Token::Str(s) => Ok(Condition::HasTag(s)),
                    _ => Err("expected string after tag".into()),
                }
            }
            Token::Ident(field) => {
                self.next();
                // operator
                let op = self.next();
                // value
                let val_tok = self.next();
                let val = match val_tok {
                    Token::Str(s) => Value::Str(s),
                    Token::Num(n) => Value::Num(n),
                    Token::Ident(b) if b == "true" => Value::Bool(true),
                    Token::Ident(b) if b == "false" => Value::Bool(false),
                    _ => return Err("expected value".into()),
                };
                match op {
                    Token::Eq => {
                        if let Token::Ident(kw) = Token::Ident("contains".into()) {
                            let _ = kw;
                        }
                        Ok(Condition::Eq(field, val))
                    }
                    Token::Ne => Ok(Condition::Ne(field, val)),
                    Token::Ident(kw) if kw == "contains" => {
                        // value must be string
                        let needle = match val {
                            Value::Str(s) => s,
                            _ => return Err("contains requires string".into()),
                        };
                        // re-read value? we already consumed. Use needle.
                        Ok(Condition::Contains(field, needle))
                    }
                    Token::Ident(kw) if kw == "has" => {
                        let needle = match val {
                            Value::Str(s) => s,
                            _ => return Err("has requires string".into()),
                        };
                        Ok(Condition::Contains(field, needle))
                    }
                    _ => Err("expected operator".into()),
                }
            }
            _ => Err("expected condition".into()),
        }
    }
}

fn parse_condition(s: &str) -> Result<Condition, String> {
    let mut p = Parser::new(s)?;
    let c = p.parse_expr()?;
    if !matches!(p.peek(), Token::Eof) {
        return Err("trailing tokens in condition".into());
    }
    Ok(c)
}

fn parse_action(s: &str) -> Result<Action, String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("set ") {
        // field = value
        let (field, rhs) = rest.split_once('=').ok_or("expected = in set")?;
        let field = field.trim().to_string();
        let val = parse_value(rhs.trim())?;
        return Ok(Action::Set(field, val));
    }
    if let Some(rest) = s.strip_prefix("add tag ") {
        let val = parse_value(rest.trim())?;
        match val {
            Value::Str(t) => return Ok(Action::AddTag(t)),
            _ => return Err("add tag requires string".into()),
        }
    }
    if let Some(rest) = s.strip_prefix("remove tag ") {
        let val = parse_value(rest.trim())?;
        match val {
            Value::Str(t) => return Ok(Action::RemoveTag(t)),
            _ => return Err("remove tag requires string".into()),
        }
    }
    if let Some(rest) = s.strip_prefix("move ") {
        let val = parse_value(rest.trim())?;
        match val {
            Value::Str(t) => return Ok(Action::Move(t)),
            _ => return Err("move requires string".into()),
        }
    }
    if s == "delete" {
        return Ok(Action::Delete);
    }
    Err(format!("unknown action: {}", s))
}

fn parse_value(s: &str) -> Result<Value, String> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"').ok_or("unterminated string")?;
        return Ok(Value::Str(rest[..end].to_string()));
    }
    if s == "true" {
        return Ok(Value::Bool(true));
    }
    if s == "false" {
        return Ok(Value::Bool(false));
    }
    if let Ok(n) = s.parse::<f64>() {
        return Ok(Value::Num(n));
    }
    // bare identifier -> string
    Ok(Value::Str(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;

    fn rec() -> Record {
        let mut r = Record::new("rec_1", "page");
        r.title = Some("cat video".to_string());
        r.status = Some("Reading".to_string());
        r.tags = vec!["rust".to_string()];
        r
    }

    #[test]
    fn parse_simple_rule() {
        let text = r#"
when status == "Reading"
then set status = "Archived"
then add tag "done"
"#;
        let rs = parse_rules(text).unwrap();
        assert_eq!(rs.rules.len(), 1);
        assert_eq!(rs.rules[0].then.len(), 2);
    }

    #[test]
    fn apply_status_change() {
        let text = r#"
when status == "Reading"
then set status = "Archived"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        let fired = apply_rules(&mut r, &rs.rules);
        assert_eq!(r.status.as_deref(), Some("Archived"));
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn contains_and_tag() {
        let text = r#"
when title contains "cat" and tag "rust"
then add tag "animal"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert!(r.tags.contains(&"animal".to_string()));
    }

    #[test]
    fn or_condition() {
        let text = r#"
when status == "Done" or tag "rust"
then set status = "Tagged"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert_eq!(r.status.as_deref(), Some("Tagged"));
    }

    #[test]
    fn not_condition() {
        let text = r#"
when not status == "Archived"
then add tag "active"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert!(r.tags.contains(&"active".to_string()));
    }

    #[test]
    fn move_action() {
        let text = r#"
when title contains "cat"
then move "Animals/Cats"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert_eq!(r.folder.as_deref(), Some("Animals/Cats"));
    }

    #[test]
    fn multiple_rules() {
        let text = r#"
rule "r1":
when tag "rust"
then add tag "systems"

when status == "Reading"
then set status = "In Progress"
"#;
        let rs = parse_rules(text).unwrap();
        assert_eq!(rs.rules.len(), 2);
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert!(r.tags.contains(&"systems".to_string()));
        assert_eq!(r.status.as_deref(), Some("In Progress"));
    }

    #[test]
    fn parse_error_missing_then() {
        let text = "when status == \"Reading\"\n";
        assert!(parse_rules(text).is_err());
    }
}
