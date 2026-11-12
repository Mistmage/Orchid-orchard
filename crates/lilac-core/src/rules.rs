//! Minimal rule engine for Lilac records.
//!
//! A rule is: `when <condition> then <action>` (possibly multiple `then`).
//! A rule may be limited to a site with `for "*.example.com"`; the scope uses
//! the same matching as naming rules (`www.` ignored, `*.` includes subdomains).
//! Conditions combine with `and` / `or` / `not` and atoms:
//!     - `true` / `false`
//!     - `tag "x"` (record has tag x)
//!     - `field == "value"` / `field != "value"`
//!     - `field contains "value"` / `field has "value"`
//!     - `field matches "*.example.com"` (case-insensitive glob, `*` and `?`)
//!     - parenthesized groups
//! Derived fields include `host` / `domain` (from the URL), `folder`, `tags`,
//! `created` and `modified`, so a per-site rule rarely needs a scope line.
//! Actions:
//!     - `set field = value`
//!     - `add tag "x"`
//!     - `remove tag "x"`
//!     - `move "folder"`
//!     - `delete`
//!     - `compute field = <expression>` (see `crate::formula`)
//!
//! `compute` expressions are parsed once with the rules; a runtime failure such
//! as division by zero is reported by `apply_rules_reported` and does not stop
//! the remaining actions.
//!
//! Syntax is intentionally minimal and subject to change; the engine is the
//! stable part.

use crate::formula::{Expr, Value as FormulaValue, eval_formula};
use crate::naming::{host_matches, normalize_host};
use crate::record::Record;
use crate::storage::host_of;

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
    Matches(String, String),
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
    /// `compute field = <expression>`; the expression was parsed at load time.
    Compute(String, Expr),
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: Option<String>,
    /// Optional host pattern; `None` or empty applies everywhere.
    pub scope: Option<String>,
    pub when: Condition,
    pub then: Vec<Action>,
    /// `false` for a rule written as `disabled rule "name": ...`. A disabled
    /// rule is parsed and listed but never applied.
    pub enabled: bool,
}

impl Action {
    /// The action in the same form the rules file uses, for previews and logs.
    pub fn label(&self) -> String {
        match self {
            Action::Set(field, value) => format!("set {} = {}", field, value.label()),
            Action::AddTag(tag) => format!("add tag {:?}", tag),
            Action::RemoveTag(tag) => format!("remove tag {:?}", tag),
            Action::Move(folder) => format!("move {:?}", folder),
            Action::Delete => "delete".to_string(),
            Action::Compute(field, _) => format!("compute {} = <expression>", field),
        }
    }
}

impl Value {
    pub fn label(&self) -> String {
        match self {
            Value::Str(s) => format!("{:?}", s),
            Value::Num(n) if n.fract() == 0.0 => format!("{}", *n as i64),
            Value::Num(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
        }
    }
}

/// One row of a dry run: what each rule would do to a record.
#[derive(Debug, Clone)]
pub struct RulePreview {
    /// Position in the rules file, zero-based.
    pub index: usize,
    pub name: Option<String>,
    pub scope: Option<String>,
    pub enabled: bool,
    /// Whether the condition matched (always false for a disabled rule).
    pub matched: bool,
    /// Actions that would run, in order.
    pub actions: Vec<String>,
    /// Actions that matched but failed at preview time.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

/// Apply rules to a record in order. Returns the actions that fired.
///
/// A rule with a `scope` only runs when the record has a URL whose host matches
/// that pattern. Records without a URL never match a scoped rule; an unscoped
/// rule runs for every record.
///
/// Errors from `compute` actions are discarded here. Callers that can show them
/// (the host log, the Orchard UI) use [`apply_rules_reported`].
pub fn apply_rules(record: &mut Record, rules: &[Rule]) -> Vec<Action> {
    apply_rules_reported(record, rules).0
}

/// Same as [`apply_rules`], but also returns human-readable errors for actions
/// that could not be applied. A broken `compute` expression never stops the
/// remaining actions or the save that triggered the rules.
pub fn apply_rules_reported(record: &mut Record, rules: &[Rule]) -> (Vec<Action>, Vec<String>) {
    let host = record.url.as_deref().and_then(host_of).map(normalize_host);
    let mut fired = Vec::new();
    let mut errors = Vec::new();
    for rule in rules {
        if !rule.enabled {
            continue;
        }
        if !scope_matches(rule, host.as_deref()) {
            continue;
        }
        if eval_condition(&rule.when, record) {
            for a in &rule.then {
                match apply_action(record, a) {
                    Ok(()) => fired.push(a.clone()),
                    Err(e) => {
                        let name = rule.name.as_deref().unwrap_or("<unnamed>");
                        errors.push(format!("rule {:?}: {}", name, e));
                    }
                }
            }
        }
    }
    (fired, errors)
}

/// Dry run: report what every rule would do to `record` without changing it.
///
/// Later rules see the changes earlier rules *would* make, so a rule that reads
/// a computed field behaves exactly as it would on a real pass. The caller's
/// record is cloned, never modified.
pub fn preview_rules(record: &Record, rules: &[Rule]) -> Vec<RulePreview> {
    let host = record.url.as_deref().and_then(host_of).map(normalize_host);
    let mut work = record.clone();
    let mut previews = Vec::with_capacity(rules.len());
    for (index, rule) in rules.iter().enumerate() {
        let mut preview = RulePreview {
            index,
            name: rule.name.clone(),
            scope: rule.scope.clone(),
            enabled: rule.enabled,
            matched: false,
            actions: Vec::new(),
            errors: Vec::new(),
        };
        if rule.enabled && scope_matches(rule, host.as_deref()) && eval_condition(&rule.when, &work)
        {
            preview.matched = true;
            for action in &rule.then {
                match apply_action(&mut work, action) {
                    Ok(()) => preview.actions.push(action.label()),
                    Err(e) => preview.errors.push(format!("{}: {}", action.label(), e)),
                }
            }
        }
        previews.push(preview);
    }
    previews
}

fn scope_matches(rule: &Rule, host: Option<&str>) -> bool {
    match rule.scope.as_deref().map(str::trim) {
        None | Some("") | Some("*") => true,
        Some(pattern) => host
            .map(|h| {
                host_matches(h, pattern)
                    || glob_matches(&normalize_host(h), &normalize_host(pattern))
            })
            .unwrap_or(false),
    }
}

/// Case-insensitive glob used by `matches`: `*` is any run of characters and
/// `?` is one character. Iterative and linear-ish, so a hostile pattern cannot
/// blow up the parser.
pub fn glob_matches(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let (mut t, mut p) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_t = 0usize;
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            t += 1;
            p += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            star_t = t;
            p += 1;
        } else if let Some(s) = star {
            p = s + 1;
            star_t += 1;
            t = star_t;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

fn apply_action(record: &mut Record, a: &Action) -> Result<(), String> {
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
        Action::Compute(field, expr) => {
            let value = eval_formula(expr, record)
                .map_err(|e| format!("compute {} failed: {}", field, e))?;
            // A list becomes its comma-separated form so typed list fields are
            // written through the same comma-splitting path as `set`.
            set_field(record, field, formula_to_rule_value(value));
        }
    }
    Ok(())
}

/// Adapt a formula value to the rule engine's value type. `null` maps to the
/// empty string, which `set_field` treats as clearing an optional field.
fn formula_to_rule_value(value: FormulaValue) -> Value {
    match value {
        FormulaValue::Null => Value::Str(String::new()),
        FormulaValue::Bool(b) => Value::Bool(b),
        FormulaValue::Num(n) => Value::Num(n),
        FormulaValue::Str(s) => Value::Str(s),
        FormulaValue::List(items) => Value::Str(
            items
                .iter()
                .map(|v| v.display())
                .collect::<Vec<_>>()
                .join(", "),
        ),
    }
}

fn value_string(val: &Value) -> String {
    match val {
        Value::Str(s) => s.clone(),
        Value::Num(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
    }
}

/// Split a comma-separated rule value into list items. Used for typed list
/// fields so `set tags = "rust, cli"` cannot write a scalar where the record
/// format expects a sequence.
fn value_list(val: &Value) -> Vec<String> {
    value_string(val)
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn value_option(val: &Value) -> Option<String> {
    let s = value_string(val);
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn set_field(record: &mut Record, field: &str, val: Value) {
    match field {
        "title" => record.title = value_option(&val),
        "status" => record.status = value_option(&val),
        "url" => record.url = value_option(&val),
        "description" => record.description = value_option(&val),
        "icon" => record.icon = value_option(&val),
        "color" => record.color = value_option(&val),
        "parent" => record.parent = value_option(&val),
        "target_type" => record.target_type = value_option(&val),
        // `folder` is the placement field, not an extra key. `move` is the
        // idiomatic action, but accepting `set folder` keeps the two consistent.
        "folder" => record.folder = value_option(&val),
        "tags" => record.tags = value_list(&val),
        "categories" => record.categories = value_list(&val),
        "related" => record.related = value_list(&val),
        "links" => record.links = value_list(&val),
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
        "target_type" => record.target_type.clone().map(Value::Str),
        "icon" => record.icon.clone().map(Value::Str),
        "color" => record.color.clone().map(Value::Str),
        "parent" => record.parent.clone().map(Value::Str),
        "categories" => Some(Value::Str(record.categories.join(", "))),
        "related" => Some(Value::Str(record.related.join(", "))),
        "links" => Some(Value::Str(record.links.join(", "))),
        "created" => record.created.clone().map(Value::Str),
        "modified" => record.modified.clone().map(Value::Str),
        "folder" => Some(Value::Str(record.folder.clone().unwrap_or_default())),
        "tags" => Some(Value::Str(record.tags.join(", "))),
        // Hosts are derived, never stored: the URL stays the only source of
        // truth, and a moved page cannot keep a stale domain field.
        "host" | "domain" | "site" => record
            .url
            .as_deref()
            .and_then(host_of)
            .map(|h| Value::Str(normalize_host(h))),
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
                // Text fields are prose, not identifiers: matching is
                // case-insensitive so "Recipe" is found by `contains "recipe"`.
                Value::Str(s) => s.to_lowercase().contains(&needle.to_lowercase()),
                _ => false,
            })
            .unwrap_or(false),
        Condition::HasTag(t) => record.tags.iter().any(|x| x == t),
        Condition::Matches(f, pattern) => get_field(record, f)
            .map(|gv| match gv {
                Value::Str(s) => glob_matches(&s, pattern),
                Value::Num(n) => glob_matches(&n.to_string(), pattern),
                Value::Bool(b) => glob_matches(&b.to_string(), pattern),
            })
            .unwrap_or(false),
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
    let mut current_scope: Option<String> = None;
    let mut current_when: Option<String> = None;
    let mut current_thens: Vec<String> = Vec::new();
    let mut current_enabled = true;

    let flush = |name: &Option<String>,
                 scope: &Option<String>,
                 w: &Option<String>,
                 thens: &Vec<String>,
                 enabled: bool,
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
            scope: scope.clone(),
            when: cond,
            then: actions,
            enabled,
        });
        Ok(())
    };

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            flush(
                &current_name,
                &current_scope,
                &current_when,
                &current_thens,
                current_enabled,
                &mut rules,
            )?;
            current_name = None;
            current_scope = None;
            current_when = None;
            current_thens.clear();
            current_enabled = true;
            continue;
        }
        let (line, disabled) = match line.strip_prefix("disabled ") {
            Some(rest) => (rest.trim_start(), true),
            None => (line, false),
        };
        if let Some(rest) = line.strip_prefix("rule ") {
            flush(
                &current_name,
                &current_scope,
                &current_when,
                &current_thens,
                current_enabled,
                &mut rules,
            )?;
            current_when = None;
            current_scope = None;
            current_thens.clear();
            current_enabled = !disabled;
            let rest = rest.trim();
            let (name, trailing) = if let Some(stripped) = rest.strip_prefix('"') {
                // A quoted name; anything after the closing quote may be a
                // `for "pattern"` clause or a colon.
                let end = stripped.find('"').ok_or("unterminated rule name")?;
                (
                    stripped[..end].to_string(),
                    stripped[end + 1..].trim().to_string(),
                )
            } else {
                (rest.to_string(), String::new())
            };
            current_name = Some(name);
            let trailing = trailing.trim_start_matches(':').trim();
            if let Some(pattern) = trailing.strip_prefix("for ") {
                current_scope = Some(parse_scope_pattern(pattern)?);
            } else if !trailing.is_empty() {
                return Err(format!("unexpected text after rule name: {}", trailing));
            }
            continue;
        }
        if disabled {
            return Err("'disabled' must be followed by 'rule \"name\":'".into());
        }
        if let Some(rest) = line.strip_prefix("for ") {
            current_scope = Some(parse_scope_pattern(rest)?);
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
    flush(
        &current_name,
        &current_scope,
        &current_when,
        &current_thens,
        current_enabled,
        &mut rules,
    )?;
    Ok(RuleSet { rules })
}

/// Parse the pattern in `for "*.example.com"`; bare words are allowed too.
fn parse_scope_pattern(raw: &str) -> Result<String, String> {
    let raw = raw.trim().trim_end_matches(':').trim();
    match parse_value(raw)? {
        Value::Str(s) => Ok(s),
        _ => Err("rule scope requires a host pattern string".into()),
    }
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
                    Token::Eq => Ok(Condition::Eq(field, val)),
                    Token::Ne => Ok(Condition::Ne(field, val)),
                    Token::Ident(kw) if kw == "contains" => {
                        // value must be string
                        let needle = match val {
                            Value::Str(s) => s,
                            _ => return Err("contains requires string".into()),
                        };
                        Ok(Condition::Contains(field, needle))
                    }
                    Token::Ident(kw) if kw == "has" => {
                        let needle = match val {
                            Value::Str(s) => s,
                            _ => return Err("has requires string".into()),
                        };
                        Ok(Condition::Contains(field, needle))
                    }
                    Token::Ident(kw) if kw == "matches" || kw == "like" => {
                        let pattern = match val {
                            Value::Str(s) => s,
                            _ => return Err("matches requires string".into()),
                        };
                        Ok(Condition::Matches(field, pattern))
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
    if let Some(rest) = s.strip_prefix("compute ") {
        // `compute field = <expression>`. The expression grammar is separate
        // (see `crate::formula`), so the split is only on the assignment `=`;
        // comparisons inside the expression use `==` and are left intact.
        let (field, rhs) = rest.split_once('=').ok_or("expected = in compute")?;
        let field = field.trim();
        if field.is_empty() {
            return Err("compute needs a field name".into());
        }
        let expr = crate::formula::parse_formula(rhs.trim())
            .map_err(|e| format!("compute {}: {}", field, e))?;
        return Ok(Action::Compute(field.to_string(), expr));
    }
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
    fn per_domain_scope_filters_by_url_host() {
        let text = r#"
rule "YouTube cleanup" for "*.youtube.com":
when tag "video"
then set status = "Video"
then add tag "video-site"

rule "GitHub only"
when tag "code"
then add tag "github-rule"
"#;
        let rs = parse_rules(text).unwrap();
        assert_eq!(rs.rules.len(), 2);
        assert_eq!(rs.rules[0].scope.as_deref(), Some("*.youtube.com"));

        let mut youtube = rec();
        youtube.tags.push("video".to_string());
        youtube.url = Some("https://www.youtube.com/watch?v=1".to_string());
        apply_rules(&mut youtube, &rs.rules);
        assert_eq!(youtube.status.as_deref(), Some("Video"));
        assert!(youtube.tags.contains(&"video-site".to_string()));

        // A scoped rule does nothing on another site...
        let mut github = rec();
        github.tags.push("video".to_string());
        github.url = Some("https://github.com/rust-lang".to_string());
        apply_rules(&mut github, &rs.rules);
        assert_eq!(github.status.as_deref(), Some("Reading"));
        // ...and a scoped rule never applies to a record with no URL.
        let mut local = rec();
        local.tags.push("video".to_string());
        apply_rules(&mut local, &rs.rules);
        assert_eq!(local.status.as_deref(), Some("Reading"));
    }

    #[test]
    fn standalone_for_line_sets_scope() {
        let text = r#"
for "example.com"
when tag "read"
then add tag "example"
"#;
        let rs = parse_rules(text).unwrap();
        assert_eq!(rs.rules[0].scope.as_deref(), Some("example.com"));
        let mut r = rec();
        r.tags.push("read".to_string());
        r.url = Some("https://example.com/a".to_string());
        apply_rules(&mut r, &rs.rules);
        assert!(r.tags.contains(&"example".to_string()));
    }

    #[test]
    fn derived_fields_read_url_and_folder() {
        let text = r#"
when host matches "*.example.com" and folder == "Inbox"
then add tag "inboxed"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        r.url = Some("https://news.example.com/post".to_string());
        r.folder = Some("Inbox".to_string());
        apply_rules(&mut r, &rs.rules);
        assert!(r.tags.contains(&"inboxed".to_string()));
    }

    #[test]
    fn glob_matching_supports_star_and_question() {
        assert!(glob_matches("YouTube.com", "*youtube*"));
        assert!(glob_matches("a.example.com", "*.example.com"));
        assert!(glob_matches("ab", "a?"));
        assert!(!glob_matches("abc", "a?"));
        assert!(glob_matches("", "*"));
        assert!(!glob_matches("abc", "b*"));
    }

    #[test]
    fn contains_is_case_insensitive() {
        let text = r#"
when title contains "recipe"
then move "Cooking"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        r.title = Some("Recipe soup".to_string());
        apply_rules(&mut r, &rs.rules);
        assert_eq!(r.folder.as_deref(), Some("Cooking"));
    }

    #[test]
    fn set_typed_and_list_fields() {
        let text = r#"
when tag "rust"
then set tags = "systems, cli"
then set folder = "Archive"
then set icon = "gear"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert_eq!(r.tags, vec!["systems".to_string(), "cli".to_string()]);
        assert_eq!(r.folder.as_deref(), Some("Archive"));
        assert_eq!(r.icon.as_deref(), Some("gear"));
        // The list is a real list, not a scalar hidden in open metadata.
        assert!(!r.extra.contains_key("tags"));
        assert!(!r.extra.contains_key("folder"));
    }

    #[test]
    fn compute_sets_text_number_and_list_fields() {
        let text = r#"
when tag "rust"
then compute summary = title + " (" + status + ")"
then compute score = 2 * 3 + 1
then compute tags = "rust, computed"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        let (fired, errors) = apply_rules_reported(&mut r, &rs.rules);
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert_eq!(fired.len(), 3);
        assert_eq!(
            r.extra.get("summary").and_then(|v| v.as_str()),
            Some("cat video (Reading)")
        );
        assert_eq!(r.extra.get("score").and_then(|v| v.as_f64()), Some(7.0));
        assert_eq!(r.tags, vec!["rust".to_string(), "computed".to_string()]);
    }

    #[test]
    fn compute_can_clear_a_field_with_null() {
        let text = r#"
when status == "Reading"
then compute status = null
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert_eq!(r.status, None);
    }

    #[test]
    fn invalid_compute_expression_is_a_parse_error() {
        let text = r#"
when true
then compute x = 1 +
"#;
        let err = parse_rules(text).unwrap_err();
        assert!(err.contains("compute"), "error: {err}");
    }

    #[test]
    fn compute_runtime_error_is_reported_and_does_not_block_later_actions() {
        let text = r#"
when true
then compute broken = 1 / 0
then add tag "after"
"#;
        let rs = parse_rules(text).unwrap();
        let mut r = rec();
        let (fired, errors) = apply_rules_reported(&mut r, &rs.rules);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("division by zero"), "{:?}", errors);
        assert!(r.tags.contains(&"after".to_string()));
        // The failing action is not reported as fired.
        assert_eq!(fired.len(), 1);
        assert!(matches!(fired[0], Action::AddTag(_)));
    }

    #[test]
    fn a_disabled_rule_is_parsed_but_never_applied() {
        let text = r#"
disabled rule "Off":
when true
then add tag "off"

rule "On":
when true
then add tag "on"
"#;
        let rs = parse_rules(text).unwrap();
        assert_eq!(rs.rules.len(), 2);
        assert!(!rs.rules[0].enabled);
        assert!(rs.rules[1].enabled);
        let mut r = rec();
        apply_rules(&mut r, &rs.rules);
        assert_eq!(r.tags, vec!["rust".to_string(), "on".to_string()]);
    }

    #[test]
    fn preview_reports_every_rule_without_mutating_the_record() {
        let text = r#"
disabled rule "Off":
when true
then add tag "off"

rule "On":
when tag "rust"
then add tag "seen"
then compute summary = title + "!"
"#;
        let rs = parse_rules(text).unwrap();
        let r = rec();
        let before = r.clone();
        let preview = preview_rules(&r, &rs.rules);
        assert_eq!(r, before, "preview must not change the record");
        assert_eq!(preview.len(), 2);
        assert!(!preview[0].enabled);
        assert!(!preview[0].matched);
        assert!(preview[0].actions.is_empty());
        assert!(preview[1].matched);
        assert_eq!(
            preview[1].actions,
            vec![
                "add tag \"seen\"".to_string(),
                "compute summary = <expression>".to_string()
            ]
        );
        assert!(preview[1].errors.is_empty());
    }

    #[test]
    fn preview_lets_later_rules_see_earlier_computed_values() {
        let text = r#"
when true
then compute status = "Computed"

when status == "Computed"
then add tag "second"
"#;
        let rs = parse_rules(text).unwrap();
        let r = rec();
        let preview = preview_rules(&r, &rs.rules);
        assert!(preview[0].matched);
        assert!(preview[1].matched);
        assert_eq!(preview[1].actions, vec!["add tag \"second\"".to_string()]);
        assert_eq!(r.status.as_deref(), Some("Reading"));
    }

    #[test]
    fn disabled_only_modifies_a_rule_header() {
        assert!(
            parse_rules("disabled when true\nthen add tag \"x\"\n")
                .unwrap_err()
                .contains("disabled")
        );
    }

    #[test]
    fn parse_error_missing_then() {
        let text = "when status == \"Reading\"\n";
        assert!(parse_rules(text).is_err());
    }
}
