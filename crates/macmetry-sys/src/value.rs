//! A small tree of values, the shape of a property list. Registry entries and property list fixtures both become a
//! [`Value`], so the parsers that read them are pure functions that tests feed with real captured data.

use std::collections::BTreeMap;

/// A property list value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Data(Vec<u8>),
    Array(Vec<Value>),
    Dict(BTreeMap<String, Value>),
}

impl Value {
    /// The entry of a dictionary.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(map) => map.get(key),
            _ => None,
        }
    }

    /// Follows a chain of dictionary keys.
    pub fn path(&self, keys: &[&str]) -> Option<&Value> {
        keys.iter().try_fold(self, |value, key| value.get(key))
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) if f.fract() == 0.0 && f.abs() < 9.0e18 => Some(*f as i64),
            _ => None,
        }
    }

    /// Counters in the registry are unsigned but arrive as signed 64-bit numbers; a negative one is not a counter.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_i64().and_then(|i| u64::try_from(i).ok())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Int(i) => Some(*i != 0),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_data(&self) -> Option<&[u8]> {
        match self {
            Value::Data(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Dict(d) => Some(d),
            _ => None,
        }
    }

    /// Converts a parsed property list, such as the output of `ioreg -a`.
    pub fn from_plist(value: plist::Value) -> Value {
        match value {
            plist::Value::Array(items) => Value::Array(items.into_iter().map(Value::from_plist).collect()),
            plist::Value::Dictionary(map) => {
                Value::Dict(map.into_iter().map(|(k, v)| (k, Value::from_plist(v))).collect())
            }
            plist::Value::Boolean(b) => Value::Bool(b),
            plist::Value::Data(d) => Value::Data(d),
            plist::Value::Real(f) => Value::Float(f),
            plist::Value::Integer(i) => match i.as_signed() {
                Some(n) => Value::Int(n),
                None => Value::Float(i.as_unsigned().unwrap_or(0) as f64),
            },
            plist::Value::String(s) => Value::Str(s),
            _ => Value::Null,
        }
    }

    /// Parses property list bytes (XML or binary).
    pub fn from_plist_bytes(bytes: &[u8]) -> crate::error::Result<Value> {
        plist::Value::from_reader(std::io::Cursor::new(bytes))
            .map(Value::from_plist)
            .map_err(|e| crate::error::SysError::parse("a property list", e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>Stats</key><dict><key>Bytes</key><integer>42</integer><key>Ratio</key><real>0.5</real></dict>
  <key>Name</key><string>disk0</string>
  <key>On</key><true/>
  <key>List</key><array><integer>1</integer><integer>2</integer></array>
  <key>Blob</key><data>AQID</data>
</dict></plist>"#;

    #[test]
    fn reads_a_property_list() {
        let v = Value::from_plist_bytes(XML).unwrap();
        assert_eq!(v.path(&["Stats", "Bytes"]).and_then(Value::as_u64), Some(42));
        assert_eq!(v.path(&["Stats", "Ratio"]).and_then(Value::as_f64), Some(0.5));
        assert_eq!(v.get("Name").and_then(Value::as_str), Some("disk0"));
        assert_eq!(v.get("On").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("List").and_then(Value::as_array).map(<[Value]>::len), Some(2));
        assert_eq!(v.get("Blob").and_then(Value::as_data), Some(&[1u8, 2, 3][..]));
    }

    #[test]
    fn a_missing_path_is_none() {
        let v = Value::from_plist_bytes(XML).unwrap();
        assert!(v.path(&["Stats", "Nope"]).is_none());
        assert!(v.path(&["Name", "Deeper"]).is_none());
        assert!(Value::Null.get("x").is_none());
    }

    #[test]
    fn a_negative_number_is_not_a_counter() {
        assert_eq!(Value::Int(-1).as_u64(), None);
        assert_eq!(Value::Int(7).as_u64(), Some(7));
        assert_eq!(Value::Float(3.0).as_i64(), Some(3));
        assert_eq!(Value::Float(3.5).as_i64(), None);
    }

    #[test]
    fn rejects_text_that_is_not_a_property_list() {
        assert!(Value::from_plist_bytes(b"hello").is_err());
    }
}
