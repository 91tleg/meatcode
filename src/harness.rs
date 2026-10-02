use crate::types::{Signature, Ty};
use std::collections::HashMap;

fn camel(s: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn py_ty(t: Ty) -> &'static str {
    match t {
        Ty::Int => "int",
        Ty::Float => "float",
        Ty::Bool => "bool",
        Ty::Str => "str",
        Ty::IntArr => "list[int]",
        Ty::FloatArr => "list[float]",
        Ty::StrArr => "list[str]",
        Ty::IntMat => "list[list[int]]",
    }
}

fn js_ty(t: Ty) -> &'static str {
    match t {
        Ty::Int | Ty::Float => "number",
        Ty::Bool => "boolean",
        Ty::Str => "string",
        Ty::IntArr | Ty::FloatArr => "number[]",
        Ty::StrArr => "string[]",
        Ty::IntMat => "number[][]",
    }
}

fn rs_ty(t: Ty) -> &'static str {
    match t {
        Ty::Int => "i64",
        Ty::Float => "f64",
        Ty::Bool => "bool",
        Ty::Str => "String",
        Ty::IntArr => "Vec<i64>",
        Ty::FloatArr => "Vec<f64>",
        Ty::StrArr => "Vec<String>",
        Ty::IntMat => "Vec<Vec<i64>>",
    }
}

fn cpp_ty(t: Ty) -> &'static str {
    match t {
        Ty::Int => "long long",
        Ty::Float => "double",
        Ty::Bool => "bool",
        Ty::Str => "string",
        Ty::IntArr => "vector<long long>",
        Ty::FloatArr => "vector<double>",
        Ty::StrArr => "vector<string>",
        Ty::IntMat => "vector<vector<long long>>",
    }
}

pub fn starters(sig: &Signature) -> Result<HashMap<String, String>, String> {
    let (ps, ret) = sig.types()?;
    let join = |f: &dyn Fn(usize, Ty) -> String| ps.iter().enumerate().map(|(i, t)| f(i, *t)).collect::<Vec<_>>().join(", ");
    let name = &sig.name;
    let mut m = HashMap::new();

    m.insert(
        "python".into(),
        format!(
            "def {name}({}) -> {}:\n    pass\n",
            join(&|i, t| format!("{}: {}", sig.params[i].name, py_ty(t))),
            py_ty(ret)
        ),
    );

    let mut doc = String::from("/**\n");
    for (i, t) in ps.iter().enumerate() {
        doc += &format!(" * @param {{{}}} {}\n", js_ty(*t), sig.params[i].name);
    }
    doc += &format!(" * @return {{{}}}\n */\n", js_ty(ret));
    m.insert(
        "javascript".into(),
        format!("{doc}function {}({}) {{\n  \n}}\n", camel(name), join(&|i, _| sig.params[i].name.clone())),
    );

    m.insert(
        "rust".into(),
        format!(
            "fn {name}({}) -> {} {{\n    todo!()\n}}\n",
            join(&|i, t| format!("{}: {}", sig.params[i].name, rs_ty(t))),
            rs_ty(ret)
        ),
    );

    m.insert(
        "cpp".into(),
        format!(
            "// Standard headers and `using namespace std;` are already included.\n{} {name}({}) {{\n    \n}}\n",
            cpp_ty(ret),
            join(&|i, t| format!("{} {}", cpp_ty(t), sig.params[i].name))
        ),
    );
    Ok(m)
}

/// Combine the user's function with a generated main that reads args from stdin and prints the result.
pub fn wrap(lang: &str, sig: &Signature, code: &str) -> Result<String, String> {
    let (ps, ret) = sig.types()?;
    let n = ps.len();
    let call_args = (0..n).map(|i| format!("a{i}")).collect::<Vec<_>>().join(", ");
    let name = &sig.name;
    Ok(match lang {
        "python" => {
            let mut s = format!("{code}\n\nimport sys as mc_sys\nmc_lines = mc_sys.stdin.read().split('\\n')\nmc_pos = 0\ndef mc_next():\n    global mc_pos\n    s = mc_lines[mc_pos] if mc_pos < len(mc_lines) else ''\n    mc_pos += 1\n    return s.rstrip('\\r')\n");
            for (i, t) in ps.iter().enumerate() {
                s += &format!("a{i} = {}\n", py_read(*t));
            }
            s += &format!("r = {name}({call_args})\n{}", py_print(ret));
            s
        }
        "javascript" => {
            let mut s = format!("{code}\n\nconst mc_lines = require('fs').readFileSync(0, 'utf8').split('\\n').map(l => l.replace(/\\r$/, ''));\nlet mc_pos = 0;\nconst mc_next = () => mc_lines[mc_pos++] ?? '';\nconst mc_nums = () => mc_next().split(/\\s+/).filter(x => x).map(Number);\n");
            for (i, t) in ps.iter().enumerate() {
                s += &format!("const a{i} = {};\n", js_read(*t));
            }
            s += &format!("const r = {}({call_args});\n{}", camel(name), js_print(ret));
            s
        }
        "rust" => {
            let mut s = format!("{code}\n\nfn main() {{\n    let mut mc_in = String::new();\n    std::io::Read::read_to_string(&mut std::io::stdin(), &mut mc_in).unwrap();\n    let mut mc_it = mc_in.split('\\n').map(|l| l.trim_end_matches('\\r'));\n    #[allow(unused_mut)]\n    let mut mc_next = || mc_it.next().unwrap_or(\"\");\n");
            for (i, t) in ps.iter().enumerate() {
                s += &format!("    let a{i}: {} = {};\n", rs_ty(*t), rs_read(*t));
            }
            s += &format!("    let r = {name}({call_args});\n{}}}\n", rs_print(ret));
            s
        }
        "cpp" => {
            let mut s = String::from("#include <iostream>\n#include <sstream>\n#include <iomanip>\n#include <vector>\n#include <string>\n#include <algorithm>\n#include <numeric>\n#include <cmath>\n#include <climits>\n#include <cstdint>\n#include <functional>\n#include <map>\n#include <set>\n#include <unordered_map>\n#include <unordered_set>\n#include <queue>\n#include <stack>\n#include <deque>\n#include <utility>\nusing namespace std;\n#line 1\n");
            s += code;
            s += "\n\nstatic string mc_ln() { string s; getline(cin, s); if (!s.empty() && s.back() == '\\r') s.pop_back(); return s; }\ntemplate <class T> static vector<T> mc_vec(const string& s) { istringstream is(s); vector<T> v; T x; while (is >> x) v.push_back(x); return v; }\nint main() {\n    cout << setprecision(15);\n";
            for (i, t) in ps.iter().enumerate() {
                s += &format!("    {} a{i} = {};\n", cpp_ty(*t), cpp_read(*t));
            }
            s += &format!("    auto r = {name}({call_args});\n{}    return 0;\n}}\n", cpp_print(ret));
            s
        }
        other => return Err(format!("unsupported language: {other}")),
    })
}

fn py_read(t: Ty) -> &'static str {
    match t {
        Ty::Int => "int(mc_next())",
        Ty::Float => "float(mc_next())",
        Ty::Bool => "mc_next().strip() == 'true'",
        Ty::Str => "mc_next()",
        Ty::IntArr => "[int(x) for x in mc_next().split()]",
        Ty::FloatArr => "[float(x) for x in mc_next().split()]",
        Ty::StrArr => "[mc_next() for _ in range(int(mc_next()))]",
        Ty::IntMat => "[[int(x) for x in mc_next().split()] for _ in range(int(mc_next()))]",
    }
}

fn py_print(t: Ty) -> &'static str {
    match t {
        Ty::Int | Ty::Str => "print(r)\n",
        Ty::Float => "print(repr(float(r)))\n",
        Ty::Bool => "print('true' if r else 'false')\n",
        Ty::IntArr => "print(' '.join(str(x) for x in r))\n",
        Ty::FloatArr => "print(' '.join(repr(float(x)) for x in r))\n",
        Ty::StrArr => "print(len(r))\nfor x in r:\n    print(x)\n",
        Ty::IntMat => "print(len(r))\nfor row in r:\n    print(' '.join(str(x) for x in row))\n",
    }
}

fn js_read(t: Ty) -> &'static str {
    match t {
        Ty::Int | Ty::Float => "Number(mc_next())",
        Ty::Bool => "mc_next().trim() === 'true'",
        Ty::Str => "mc_next()",
        Ty::IntArr | Ty::FloatArr => "mc_nums()",
        Ty::StrArr => "Array.from({ length: Number(mc_next()) }, () => mc_next())",
        Ty::IntMat => "Array.from({ length: Number(mc_next()) }, () => mc_nums())",
    }
}

fn js_print(t: Ty) -> &'static str {
    match t {
        Ty::Int | Ty::Float | Ty::Str => "console.log(String(r));\n",
        Ty::Bool => "console.log(r ? 'true' : 'false');\n",
        Ty::IntArr | Ty::FloatArr => "console.log(r.join(' '));\n",
        Ty::StrArr => "console.log(r.length);\nfor (const x of r) console.log(x);\n",
        Ty::IntMat => "console.log(r.length);\nfor (const row of r) console.log(row.join(' '));\n",
    }
}

fn rs_read(t: Ty) -> &'static str {
    match t {
        Ty::Int => "mc_next().trim().parse::<i64>().unwrap()",
        Ty::Float => "mc_next().trim().parse::<f64>().unwrap()",
        Ty::Bool => "mc_next().trim() == \"true\"",
        Ty::Str => "mc_next().to_string()",
        Ty::IntArr => "mc_next().split_whitespace().map(|x| x.parse::<i64>().unwrap()).collect()",
        Ty::FloatArr => "mc_next().split_whitespace().map(|x| x.parse::<f64>().unwrap()).collect()",
        Ty::StrArr => "{ let n: usize = mc_next().trim().parse().unwrap(); (0..n).map(|_| mc_next().to_string()).collect() }",
        Ty::IntMat => "{ let n: usize = mc_next().trim().parse().unwrap(); (0..n).map(|_| mc_next().split_whitespace().map(|x| x.parse::<i64>().unwrap()).collect()).collect() }",
    }
}

fn rs_print(t: Ty) -> &'static str {
    match t {
        Ty::Int | Ty::Bool | Ty::Str => "    println!(\"{}\", r);\n",
        Ty::Float => "    println!(\"{:?}\", r);\n",
        Ty::IntArr => "    println!(\"{}\", r.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(\" \"));\n",
        Ty::FloatArr => "    println!(\"{}\", r.iter().map(|x| format!(\"{:?}\", x)).collect::<Vec<_>>().join(\" \"));\n",
        Ty::StrArr => "    println!(\"{}\", r.len());\n    for x in &r { println!(\"{}\", x); }\n",
        Ty::IntMat => "    println!(\"{}\", r.len());\n    for row in &r { println!(\"{}\", row.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(\" \")); }\n",
    }
}

fn cpp_read(t: Ty) -> &'static str {
    match t {
        Ty::Int => "stoll(mc_ln())",
        Ty::Float => "stod(mc_ln())",
        Ty::Bool => "mc_ln() == \"true\"",
        Ty::Str => "mc_ln()",
        Ty::IntArr => "mc_vec<long long>(mc_ln())",
        Ty::FloatArr => "mc_vec<double>(mc_ln())",
        Ty::StrArr => "[&] { int n = stoi(mc_ln()); vector<string> v; for (int i = 0; i < n; i++) v.push_back(mc_ln()); return v; }()",
        Ty::IntMat => "[&] { int n = stoi(mc_ln()); vector<vector<long long>> v; for (int i = 0; i < n; i++) v.push_back(mc_vec<long long>(mc_ln())); return v; }()",
    }
}

fn cpp_print(t: Ty) -> &'static str {
    match t {
        Ty::Int | Ty::Float | Ty::Str => "    cout << r << \"\\n\";\n",
        Ty::Bool => "    cout << (r ? \"true\" : \"false\") << \"\\n\";\n",
        Ty::IntArr | Ty::FloatArr => "    for (size_t i = 0; i < r.size(); i++) cout << (i ? \" \" : \"\") << r[i];\n    cout << \"\\n\";\n",
        Ty::StrArr => "    cout << r.size() << \"\\n\";\n    for (auto& x : r) cout << x << \"\\n\";\n",
        Ty::IntMat => "    cout << r.size() << \"\\n\";\n    for (auto& row : r) { for (size_t i = 0; i < row.size(); i++) cout << (i ? \" \" : \"\") << row[i]; cout << \"\\n\"; }\n",
    }
}
