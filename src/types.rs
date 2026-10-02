use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    IntArr,
    FloatArr,
    StrArr,
    IntMat,
}

impl Ty {
    pub fn parse(s: &str) -> Result<Ty, String> {
        Ok(match s {
            "int" => Ty::Int,
            "float" => Ty::Float,
            "bool" => Ty::Bool,
            "string" => Ty::Str,
            "int[]" => Ty::IntArr,
            "float[]" => Ty::FloatArr,
            "string[]" => Ty::StrArr,
            "int[][]" => Ty::IntMat,
            _ => return Err(format!("unknown type '{s}' (use int, float, bool, string, int[], float[], string[], int[][])")),
        })
    }

    /// Serialize a JSON value into the line protocol the harness reads from stdin.
    pub fn encode(self, v: &Value) -> Result<String, String> {
        let bad = || format!("value {v} is not a valid {self:?}");
        let ints = |v: &Value| -> Result<String, String> {
            let a = v.as_array().ok_or_else(bad)?;
            let xs: Result<Vec<String>, String> =
                a.iter().map(|x| x.as_i64().map(|n| n.to_string()).ok_or_else(bad)).collect();
            Ok(xs?.join(" "))
        };
        Ok(match self {
            Ty::Int => format!("{}\n", v.as_i64().ok_or_else(bad)?),
            Ty::Float => format!("{}\n", v.as_f64().ok_or_else(bad)?),
            Ty::Bool => format!("{}\n", v.as_bool().ok_or_else(bad)?),
            Ty::Str => {
                let s = v.as_str().ok_or_else(bad)?;
                if s.contains('\n') {
                    return Err("strings containing newlines are not supported".into());
                }
                format!("{s}\n")
            }
            Ty::IntArr => format!("{}\n", ints(v)?),
            Ty::FloatArr => {
                let a = v.as_array().ok_or_else(bad)?;
                let xs: Result<Vec<String>, String> =
                    a.iter().map(|x| x.as_f64().map(|n| n.to_string()).ok_or_else(bad)).collect();
                format!("{}\n", xs?.join(" "))
            }
            Ty::StrArr => {
                let a = v.as_array().ok_or_else(bad)?;
                let mut out = format!("{}\n", a.len());
                for x in a {
                    out += &Ty::Str.encode(x)?;
                }
                out
            }
            Ty::IntMat => {
                let a = v.as_array().ok_or_else(bad)?;
                let mut out = format!("{}\n", a.len());
                for row in a {
                    out += &format!("{}\n", ints(row)?);
                }
                out
            }
        })
    }

    /// Parse the harness's stdout back into JSON. None if the output is malformed.
    pub fn decode(self, text: &str) -> Option<Value> {
        let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
        let first = *lines.first()?;
        let ints = |s: &str| -> Option<Value> {
            s.split_whitespace().map(|x| x.parse::<i64>().ok().map(Value::from)).collect::<Option<Vec<_>>>().map(Value::Array)
        };
        match self {
            Ty::Int => first.trim().parse::<i64>().ok().map(Value::from),
            Ty::Float => first.trim().parse::<f64>().ok().and_then(Number::from_f64).map(Value::Number),
            Ty::Bool => match first.trim() {
                "true" => Some(Value::Bool(true)),
                "false" => Some(Value::Bool(false)),
                _ => None,
            },
            Ty::Str => Some(Value::String(first.to_string())),
            Ty::IntArr => ints(first),
            Ty::FloatArr => first
                .split_whitespace()
                .map(|x| x.parse::<f64>().ok().and_then(Number::from_f64).map(Value::Number))
                .collect::<Option<Vec<_>>>()
                .map(Value::Array),
            Ty::StrArr => {
                let n: usize = first.trim().parse().ok()?;
                let rows = lines.get(1..1 + n)?;
                Some(Value::Array(rows.iter().map(|s| Value::String(s.to_string())).collect()))
            }
            Ty::IntMat => {
                let n: usize = first.trim().parse().ok()?;
                let rows = lines.get(1..1 + n)?;
                rows.iter().map(|r| ints(r)).collect::<Option<Vec<_>>>().map(Value::Array)
            }
        }
    }
}

/// Structural equality with a small tolerance for floating-point numbers.
pub fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_i64(), y.as_i64()) {
            (Some(p), Some(q)) => p == q,
            _ => {
                let (p, q) = (x.as_f64().unwrap_or(f64::NAN), y.as_f64().unwrap_or(f64::NAN));
                (p - q).abs() <= 1e-6 * q.abs().max(1.0)
            }
        },
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equal(p, q)),
        _ => a == b,
    }
}

/// Sort a top-level array so answers that may come back in any order compare equal.
pub fn sorted(v: &Value) -> Value {
    match v {
        Value::Array(a) => {
            let mut a = a.clone();
            a.sort_by_key(|x| x.to_string());
            Value::Array(a)
        }
        other => other.clone(),
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Signature {
    pub name: String, // snake_case; converted to camelCase for JavaScript
    pub params: Vec<Param>,
    pub returns: String,
}

impl Signature {
    pub fn types(&self) -> Result<(Vec<Ty>, Ty), String> {
        let ps = self.params.iter().map(|p| Ty::parse(&p.ty)).collect::<Result<Vec<_>, _>>()?;
        Ok((ps, Ty::parse(&self.returns)?))
    }
}
