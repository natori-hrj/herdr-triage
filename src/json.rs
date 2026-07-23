//! A minimal JSON reader — just enough to consume herdr's `agent.list` and a config file.
//!
//! Why hand-rolled: the payloads are two known shapes from two known producers, and reading
//! them needs a handful of string and number fields. serde + serde_json would be ~15 crates
//! for that. This crate takes a dependency only where std genuinely cannot do the job, and
//! this is not one of those places. (herdr-lazy carries the same reader for the same reason;
//! the two are kept deliberately similar so a fix in one transfers to the other.)
//!
//! Why a real parser rather than scanning for `"agent_status":"..."`: an `agent.list`
//! response nests one object per agent inside `result.agents[]`, and the response envelope
//! reuses key names (`id` at the top level, `terminal_id`/`pane_id` inside). Substring
//! scanning reads whichever comes first. Parsing structurally is the only way to ask for
//! *this* agent's field and get the right answer.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(BTreeMap<String, Value>),
}

impl Value {
    /// Field of an object, or None for any other shape.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.get(key),
            _ => None,
        }
    }

    /// Follow a chain of object keys: `v.path(&["result", "agents"])`.
    pub fn path(&self, keys: &[&str]) -> Option<&Value> {
        keys.iter().try_fold(self, |v, k| v.get(k))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Convenience: a string field of this object.
    ///
    /// Returns None both when the field is absent and when it is `null`. Many `AgentInfo`
    /// fields are nullable, and for every one Triage reads, "absent" and "null" mean the
    /// same thing to us.
    pub fn str_field(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    /// Convenience: a numeric field of this object.
    pub fn num_field(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut p = Parser { b: &chars, i: 0 };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("trailing input at char {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [char],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.b.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.i += 1;
        }
        c
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        match self.bump() {
            Some(got) if got == c => Ok(()),
            Some(got) => Err(format!(
                "expected `{}`, found `{}` at {}",
                c,
                got,
                self.i - 1
            )),
            None => Err(format!("expected `{}`, found end of input", c)),
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, String> {
        if self.b[self.i..].starts_with(&word.chars().collect::<Vec<_>>()[..]) {
            self.i += word.chars().count();
            Ok(v)
        } else {
            Err(format!("invalid literal at {}", self.i))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Value::Str(self.string()?)),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some(_) => self.number(),
            None => Err("unexpected end of input".to_string()),
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut map = BTreeMap::new();
        self.ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Value::Obj(map));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            self.expect(':')?;
            self.ws();
            let v = self.value()?;
            map.insert(k, v);
            self.ws();
            match self.bump() {
                Some(',') => continue,
                Some('}') => return Ok(Value::Obj(map)),
                _ => return Err(format!("malformed object near {}", self.i)),
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Value::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.bump() {
                Some(',') => continue,
                Some(']') => return Ok(Value::Arr(items)),
                _ => return Err(format!("malformed array near {}", self.i)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return Err("unterminated string".to_string()),
                Some('"') => return Ok(s),
                Some('\\') => match self.bump() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('b') => s.push('\u{8}'),
                    Some('f') => s.push('\u{c}'),
                    Some('u') => s.push(self.unicode_escape()?),
                    other => return Err(format!("bad escape `\\{:?}`", other)),
                },
                Some(c) => s.push(c),
            }
        }
    }

    /// `\uXXXX`, including surrogate pairs. Lone surrogates become U+FFFD rather than
    /// failing the whole parse — we would rather render an agent list with one odd title
    /// than reject it outright.
    fn unicode_escape(&mut self) -> Result<char, String> {
        let hi = self.hex4()?;
        if (0xD800..0xDC00).contains(&hi) {
            // High surrogate: expect a following `\uXXXX` low surrogate.
            if self.peek() == Some('\\') && self.b.get(self.i + 1) == Some(&'u') {
                self.i += 2;
                let lo = self.hex4()?;
                if (0xDC00..0xE000).contains(&lo) {
                    let c = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                    return Ok(char::from_u32(c).unwrap_or('\u{FFFD}'));
                }
            }
            return Ok('\u{FFFD}');
        }
        Ok(char::from_u32(hi).unwrap_or('\u{FFFD}'))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut n = 0u32;
        for _ in 0..4 {
            let c = self.bump().ok_or("truncated \\u escape")?;
            let d = c.to_digit(16).ok_or(format!("bad hex digit `{}`", c))?;
            n = n * 16 + d;
        }
        Ok(n)
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()
            || c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E')
        {
            self.i += 1;
        }
        let s: String = self.b[start..self.i].iter().collect();
        s.parse::<f64>()
            .map(Value::Num)
            .map_err(|_| format!("bad number `{}`", s))
    }
}

/// Write a JSON string literal, quotes and escapes included.
///
/// Only needed for outbound requests, whose only string values are a method name and an id
/// we choose ourselves — but escaping them properly costs four lines and removes the
/// question of what happens if that ever stops being true.
pub fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact payload shape herdr 0.7.5 returns for `agent.list`, including the nullable
    /// fields the TypeScript version used to type as non-null strings.
    const REAL_LIST: &str = r#"{"id":"1","result":{"agents":[{"agent":"claude","agent_status":"blocked","cwd":"/w","focused":false,"pane_id":"w1:pA","revision":7,"state_change_seq":3,"tab_id":"w1:t1","terminal_id":"term-1","terminal_title_stripped":"Migrate the auth module","workspace_id":"w1"},{"agent":null,"agent_status":"unknown","cwd":null,"focused":true,"pane_id":"w1:pB","revision":2,"tab_id":"w1:t1","terminal_id":"term-2","terminal_title_stripped":null,"workspace_id":"w1"}],"type":"agent_list"}}"#;

    #[test]
    fn reads_agent_fields_from_real_payload() {
        let v = parse(REAL_LIST).expect("should parse");
        let agents = v.path(&["result", "agents"]).unwrap().as_array().unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].str_field("terminal_id"), Some("term-1"));
        assert_eq!(agents[0].str_field("agent_status"), Some("blocked"));
        assert_eq!(agents[0].num_field("state_change_seq"), Some(3.0));
    }

    /// Nullable fields read as None rather than as the string "null" or a parse failure.
    #[test]
    fn null_fields_read_as_absent() {
        let v = parse(REAL_LIST).unwrap();
        let agents = v.path(&["result", "agents"]).unwrap().as_array().unwrap();
        assert_eq!(agents[1].str_field("agent"), None);
        assert_eq!(agents[1].str_field("cwd"), None);
        assert_eq!(agents[1].str_field("terminal_title_stripped"), None);
        assert_eq!(agents[1].str_field("agent_status"), Some("unknown"));
    }

    /// The bug a real parser exists to prevent: the envelope's own `id` must not be mistaken
    /// for an agent's `terminal_id`, which naive substring scanning does.
    #[test]
    fn envelope_fields_do_not_leak_into_agents() {
        let v = parse(REAL_LIST).unwrap();
        assert_eq!(v.str_field("id"), Some("1"));
        let a = &v.path(&["result", "agents"]).unwrap().as_array().unwrap()[0];
        assert_eq!(a.str_field("id"), None, "an agent has no bare `id` field");
    }

    #[test]
    fn empty_agent_list() {
        let v = parse(r#"{"id":"1","result":{"agents":[],"type":"agent_list"}}"#).unwrap();
        assert!(v
            .path(&["result", "agents"])
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn scalars_and_escapes() {
        assert_eq!(parse("null").unwrap(), Value::Null);
        assert_eq!(parse(" true ").unwrap(), Value::Bool(true));
        assert_eq!(parse("-12.5e2").unwrap(), Value::Num(-1250.0));
        assert_eq!(
            parse(r#""a\"b\\c\nd""#).unwrap(),
            Value::Str("a\"b\\c\nd".to_string())
        );
        // Literal (unescaped) multi-byte input, at 3 and 4 bytes per character: the parser
        // walks `chars()`, so anything indexing by byte would split these and panic.
        assert_eq!(parse(r#""あ""#).unwrap(), Value::Str("あ".to_string()));
        assert_eq!(parse(r#""🚀""#).unwrap(), Value::Str("🚀".to_string()));
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(parse("{").is_err());
        assert!(parse(r#"{"a":1,}"#).is_err());
        assert!(parse(r#"{"a":1} junk"#).is_err());
        assert!(parse(r#""unterminated"#).is_err());
    }

    #[test]
    fn writes_escaped_strings() {
        let mut s = String::new();
        write_str(&mut s, "a\"b\\c\nd");
        assert_eq!(s, r#""a\"b\\c\nd""#);
    }
}
