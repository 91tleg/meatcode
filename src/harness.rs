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

/// C signature pieces. Fixed-width types, no heap. Inputs are read-only and carry their
/// length (`xs_len`; matrices `m_rows` plus per-row `m_cols`). Array and string results are written into a
/// caller-provided buffer (`ret`, `ret_cap`) and the function returns how many items it wrote.
fn c_scalar(t: Ty) -> &'static str {
    match t {
        Ty::Int => "int64_t",
        Ty::Float => "double",
        Ty::Bool => "bool",
        Ty::Str => "const char*",
        Ty::IntArr => "const int64_t*",
        Ty::FloatArr => "const double*",
        Ty::StrArr => "const char* const*",
        Ty::IntMat => "const int64_t* const*",
    }
}

fn c_params(sig: &Signature, ps: &[Ty], ret: Ty) -> String {
    let mut v: Vec<String> = Vec::new();
    for (p, t) in sig.params.iter().zip(ps) {
        let n = &p.name;
        v.push(format!("{} {n}", c_scalar(*t)));
        match t {
            Ty::IntArr | Ty::FloatArr | Ty::StrArr => v.push(format!("int32_t {n}_len")),
            Ty::IntMat => {
                v.push(format!("int32_t {n}_rows"));
                v.push(format!("const int32_t* {n}_cols"));
            }
            _ => {}
        }
    }
    match ret {
        Ty::IntArr => v.push("int64_t* ret, int32_t ret_cap".into()),
        Ty::FloatArr => v.push("double* ret, int32_t ret_cap".into()),
        Ty::Str | Ty::StrArr => v.push("char* ret, int32_t ret_cap".into()),
        Ty::IntMat => v.push("int64_t* ret, int32_t ret_cap, int32_t* ret_cols, int32_t ret_rows_cap".into()),
        _ => {}
    }
    v.join(", ")
}

fn c_ret(t: Ty) -> &'static str {
    match t {
        Ty::Int => "int64_t",
        Ty::Float => "double",
        Ty::Bool => "bool",
        _ => "int32_t",
    }
}

fn c_starter(sig: &Signature, ps: &[Ty], ret: Ty) -> String {
    let out = match ret {
        Ty::IntArr | Ty::FloatArr => "// Write the result into `ret` (room for `ret_cap` elements) and return how many you wrote.\n",
        Ty::Str => "// Write the characters into `ret` (room for `ret_cap`, no terminator needed) and return the length.\n",
        Ty::StrArr => "// Write the strings back to back into `ret`, each NUL-terminated (room for `ret_cap` chars), and return how many.\n",
        Ty::IntMat => "// Write all rows back to back (row-major) into `ret` and each row's width into `ret_cols[i]`;\n// return the number of rows (room for `ret_cap` elements and `ret_rows_cap` rows).\n",
        _ => "",
    };
    format!(
        "// No heap: malloc/calloc/realloc/free are disabled. Fixed-width types come from <stdint.h>.\n// Inputs are read-only and carry their length (`xs_len`); matrices as `m_rows` plus per-row `m_cols`.\n{out}{} {}({}) {{\n    \n}}\n",
        c_ret(ret),
        sig.name,
        c_params(sig, ps, ret)
    )
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

    m.insert("c".into(), c_starter(sig, &ps, ret));
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
        "c" => {
            // The harness helpers come first and may allocate; the ban applies from the user's code onward.
            let mut s = String::from(C_PRELUDE);
            s += "// The solution must not allocate. Using any of these is a compile error.\n#pragma GCC poison malloc calloc realloc free strdup strndup getline getdelim asprintf aligned_alloc posix_memalign alloca\n#line 1\n";
            s += code;
            s += "\n\nint main(void) {\n";
            let mut args: Vec<String> = Vec::new();
            for (i, t) in ps.iter().enumerate() {
                match t {
                    Ty::Int => s += &format!("    int64_t a{i} = mc_scalar_i64();\n"),
                    Ty::Float => s += &format!("    double a{i} = strtod(mc_ln(), NULL);\n"),
                    Ty::Bool => s += &format!("    bool a{i} = strncmp(mc_ln(), \"true\", 4) == 0;\n"),
                    Ty::Str => s += &format!("    const char* a{i} = mc_str_copy(mc_ln());\n"),
                    Ty::IntArr => s += &format!("    int32_t n{i}; const int64_t* a{i} = mc_ints(mc_ln(), &n{i});\n"),
                    Ty::FloatArr => s += &format!("    int32_t n{i}; const double* a{i} = mc_dbls(mc_ln(), &n{i});\n"),
                    Ty::StrArr => s += &format!("    int32_t n{i}; const char* const* a{i} = mc_strs(&n{i});\n"),
                    Ty::IntMat => s += &format!("    int32_t n{i}; const int32_t* c{i}; const int64_t* const* a{i} = mc_mat(&n{i}, &c{i});\n"),
                }
                args.push(match t {
                    Ty::IntArr | Ty::FloatArr | Ty::StrArr => format!("a{i}, n{i}"),
                    Ty::IntMat => format!("a{i}, n{i}, c{i}"),
                    _ => format!("a{i}"),
                });
            }
            match ret {
                Ty::IntArr => args.push("mc_oi, MC_ELEMS".into()),
                Ty::FloatArr => args.push("mc_od, MC_ELEMS".into()),
                Ty::Str | Ty::StrArr => args.push("mc_os, MC_ELEMS".into()),
                Ty::IntMat => args.push("mc_oi, MC_ELEMS, mc_ocols, MC_ROWS".into()),
                _ => {}
            }
            s += &format!("    {} r = {name}({});\n", c_ret(ret), args.join(", "));
            s += c_print(ret);
            s += "    return 0;\n}\n";
            s
        }
        other => return Err(format!("unsupported language: {other}")),
    })
}

/// Harness side of the C runner: parsing helpers (free to allocate, nothing is freed since the process exits after
/// one test) and the fixed output buffers handed to the solution, which may not allocate.
const C_PRELUDE: &str = r#"#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <limits.h>
#include <float.h>
#include <math.h>
#include <ctype.h>
#include <assert.h>
#include <errno.h>
#include <stddef.h>

#define MC_ELEMS (1 << 20)
#define MC_ROWS (1 << 18)
static int64_t mc_oi[MC_ELEMS];
static double mc_od[MC_ELEMS];
static char mc_os[MC_ELEMS];
static int32_t mc_ocols[MC_ROWS];

static void mc_fail(const char* m) { fprintf(stderr, "meatcode harness: %s\n", m); exit(2); }
static void* mc_alloc(size_t n) { void* p = malloc(n ? n : 1); if (!p) mc_fail("out of memory"); return p; }

static char* mc_buf; static size_t mc_cap;
static const char* mc_ln(void) {
    ssize_t n = getline(&mc_buf, &mc_cap, stdin);
    if (n < 0) { if (!mc_buf) mc_buf = mc_alloc(1); mc_buf[0] = 0; return mc_buf; }
    while (n > 0 && (mc_buf[n - 1] == '\n' || mc_buf[n - 1] == '\r')) mc_buf[--n] = 0;
    return mc_buf;
}
static bool mc_next_i64(const char** p, int64_t* out) {
    const char* s = *p;
    while (*s == ' ' || *s == '\t') s++;
    bool neg = false;
    if (*s == '-') { neg = true; s++; } else if (*s == '+') s++;
    if (*s < '0' || *s > '9') return false;
    uint64_t v = 0;
    while (*s >= '0' && *s <= '9') { v = v * 10u + (uint64_t)(*s - '0'); s++; }
    *out = neg ? (int64_t)(0u - v) : (int64_t)v;
    *p = s;
    return true;
}
static int64_t mc_scalar_i64(void) { const char* s = mc_ln(); int64_t v = 0; mc_next_i64(&s, &v); return v; }
static const int64_t* mc_ints(const char* s, int32_t* len) {
    int32_t n = 0; const char* t = s; int64_t v;
    while (mc_next_i64(&t, &v)) n++;
    int64_t* a = mc_alloc((size_t)n * sizeof *a);
    t = s;
    for (int32_t i = 0; i < n; i++) mc_next_i64(&t, &a[i]);
    *len = n;
    return a;
}
static const double* mc_dbls(const char* s, int32_t* len) {
    int32_t n = 0; char* e; const char* t = s;
    for (;;) { strtod(t, &e); if (e == t) break; n++; t = e; }
    double* a = mc_alloc((size_t)n * sizeof *a);
    t = s;
    for (int32_t i = 0; i < n; i++) { a[i] = strtod(t, &e); t = e; }
    *len = n;
    return a;
}
static const char* mc_str_copy(const char* s) { size_t n = strlen(s) + 1; char* d = mc_alloc(n); memcpy(d, s, n); return d; }
static const char* const* mc_strs(int32_t* len) {
    int32_t n = (int32_t)mc_scalar_i64();
    const char** a = mc_alloc((size_t)n * sizeof *a);
    for (int32_t i = 0; i < n; i++) a[i] = mc_str_copy(mc_ln());
    *len = n;
    return a;
}
static const int64_t* const* mc_mat(int32_t* rows, const int32_t** cols) {
    int32_t n = (int32_t)mc_scalar_i64();
    const int64_t** rp = mc_alloc((size_t)n * sizeof *rp);
    int32_t* cp = mc_alloc((size_t)n * sizeof *cp);
    for (int32_t i = 0; i < n; i++) rp[i] = mc_ints(mc_ln(), &cp[i]);
    *rows = n;
    *cols = cp;
    return rp;
}
static void mc_check(int32_t n, int32_t cap) { if (n < 0 || n > cap) mc_fail("the function returned a length outside its output buffer"); }

"#;

fn c_print(t: Ty) -> &'static str {
    match t {
        Ty::Int => "    printf(\"%\" PRId64 \"\\n\", r);\n",
        Ty::Float => "    printf(\"%.17g\\n\", r);\n",
        Ty::Bool => "    printf(\"%s\\n\", r ? \"true\" : \"false\");\n",
        Ty::Str => "    mc_check(r, MC_ELEMS);\n    fwrite(mc_os, 1, (size_t)r, stdout);\n    printf(\"\\n\");\n",
        Ty::IntArr => "    mc_check(r, MC_ELEMS);\n    for (int32_t i = 0; i < r; i++) printf(\"%s%\" PRId64, i ? \" \" : \"\", mc_oi[i]);\n    printf(\"\\n\");\n",
        Ty::FloatArr => "    mc_check(r, MC_ELEMS);\n    for (int32_t i = 0; i < r; i++) printf(\"%s%.17g\", i ? \" \" : \"\", mc_od[i]);\n    printf(\"\\n\");\n",
        Ty::StrArr => "    mc_check(r, MC_ELEMS);\n    printf(\"%d\\n\", (int)r);\n    { size_t off = 0; for (int32_t i = 0; i < r; i++) { const char* e = off < MC_ELEMS ? memchr(mc_os + off, 0, MC_ELEMS - off) : NULL; if (!e) mc_fail(\"a returned string is not NUL-terminated inside the buffer\"); printf(\"%s\\n\", mc_os + off); off = (size_t)(e - mc_os) + 1; } }\n",
        Ty::IntMat => "    mc_check(r, MC_ROWS);\n    printf(\"%d\\n\", (int)r);\n    { size_t off = 0; for (int32_t i = 0; i < r; i++) { int32_t w = mc_ocols[i]; if (w < 0 || off + (size_t)w > MC_ELEMS) mc_fail(\"a returned row is outside the output buffer\"); for (int32_t j = 0; j < w; j++) printf(\"%s%\" PRId64, j ? \" \" : \"\", mc_oi[off + (size_t)j]); printf(\"\\n\"); off += (size_t)w; } }\n",
    }
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
