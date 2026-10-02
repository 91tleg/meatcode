import { useEffect, useState } from 'react'
import Editor from '@monaco-editor/react'
import Markdown from 'react-markdown'

type Summary = { slug: string; title: string; difficulty: string }
type Test = { args: unknown[]; expected: unknown; hidden?: boolean }
type Problem = Summary & {
  description: string
  starter: Record<string, string>
  function: { params: { name: string; type: string }[] }
  tests: Test[]
}
type TestResult = {
  passed: boolean; hidden: boolean; input: string; expected: string
  actual: string; stderr: string; status: string; ms: number
}
type RunResult = { compile_error: string | null; results: TestResult[]; passed: number; total: number }

const LANGS = [
  { id: 'python', label: 'Python', monaco: 'python' },
  { id: 'javascript', label: 'JavaScript', monaco: 'javascript' },
  { id: 'rust', label: 'Rust', monaco: 'rust' },
  { id: 'cpp', label: 'C++', monaco: 'cpp' },
]

export default function App() {
  const [list, setList] = useState<Summary[]>([])
  const [slug, setSlug] = useState<string | null>(null)
  const [problem, setProblem] = useState<Problem | null>(null)
  const [lang, setLang] = useState(localStorage.getItem('lang') || 'python')
  const [code, setCode] = useState('')
  const [result, setResult] = useState<RunResult | null>(null)
  const [busy, setBusy] = useState(false)
  const [mode, setMode] = useState<'run' | 'submit'>('run')
  const [submitted, setSubmitted] = useState<{ code: string; lang: string } | null>(null)
  const [review, setReview] = useState<string | null>(null)
  const [reviewing, setReviewing] = useState(false)
  const [reviewErr, setReviewErr] = useState<string | null>(null)

  useEffect(() => {
    fetch('/api/problems').then(r => r.json()).then((l: Summary[]) => {
      setList(l)
      if (l.length) setSlug(l[0].slug)
    })
  }, [])

  useEffect(() => {
    if (!slug) return
    setResult(null); setSubmitted(null); setReview(null); setReviewErr(null)
    fetch(`/api/problems/${slug}`).then(r => r.json()).then(setProblem)
  }, [slug])

  // Load saved draft for this problem+language, else the starter.
  useEffect(() => {
    if (!problem) return
    setCode(localStorage.getItem(`code:${problem.slug}:${lang}`) ?? problem.starter[lang] ?? '')
  }, [problem, lang])

  const onCode = (v?: string) => {
    const s = v ?? ''
    setCode(s)
    if (problem) localStorage.setItem(`code:${problem.slug}:${lang}`, s)
  }

  const reset = () => {
    if (!problem) return
    localStorage.removeItem(`code:${problem.slug}:${lang}`)
    setCode(problem.starter[lang] ?? '')
  }

  const run = async (submit: boolean) => {
    if (!problem) return
    setBusy(true); setMode(submit ? 'submit' : 'run'); setResult(null)
    setReview(null); setReviewErr(null); setSubmitted(submit ? { code, lang } : null)
    try {
      const r = await fetch(`/api/problems/${problem.slug}/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ language: lang, code, submit }),
      })
      setResult(await r.json())
    } finally { setBusy(false) }
  }

  const askReview = async () => {
    if (!problem || !submitted || !result) return
    setReviewing(true); setReview(null); setReviewErr(null)
    try {
      const r = await fetch(`/api/problems/${problem.slug}/review`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ language: submitted.lang, code: submitted.code, result }),
      })
      if (r.ok) setReview((await r.json()).review)
      else setReviewErr(await r.text())
    } catch (e) { setReviewErr(String(e)) }
    finally { setReviewing(false) }
  }

  return (
    <div className="app">
      <header className="topbar">
        <h1>meatcode</h1>
        <select className="picker" value={slug ?? ''} onChange={e => setSlug(e.target.value)}>
          {list.length === 0 && <option value="">No problems yet</option>}
          {list.map(p => <option key={p.slug} value={p.slug}>{p.title} · {p.difficulty}</option>)}
        </select>
      </header>
      <main className="main">

      {problem ? (
        <>
          <section className="desc">
            <h2>{problem.title} <span className={'diff ' + problem.difficulty.toLowerCase()}>{problem.difficulty}</span></h2>
            <div className="prose">{problem.description}</div>
            {problem.tests.map((t, i) => (
              <div key={i} className="example">
                <b>Example {i + 1}</b>
                <pre>Input:{'\n'}{problem.function.params.map((p, j) => `${p.name} = ${JSON.stringify(t.args[j])}`).join('\n')}</pre>
                <pre>Output:{'\n'}{JSON.stringify(t.expected)}</pre>
              </div>
            ))}
          </section>

          <section className="work">
            <div className="toolbar">
              <select value={lang} onChange={e => { setLang(e.target.value); localStorage.setItem('lang', e.target.value) }}>
                {LANGS.map(l => <option key={l.id} value={l.id}>{l.label}</option>)}
              </select>
              <button onClick={reset} className="ghost">Reset</button>
              <span className="spacer" />
              <button disabled={busy} onClick={() => run(false)}>Run</button>
              <button disabled={busy} onClick={() => run(true)} className="primary">Submit</button>
            </div>
            <Editor
              height="55%" theme="vs-dark" value={code} onChange={onCode}
              language={LANGS.find(l => l.id === lang)?.monaco}
              options={{ minimap: { enabled: false }, fontSize: 14, automaticLayout: true }}
            />
            <div className="results">
              {busy && <p className="muted">Running…</p>}
              {result?.compile_error && <pre className="err">{result.compile_error}</pre>}
              {result && !result.compile_error && (
                <>
                  <p className={result.passed === result.total ? 'ok' : 'bad'}>
                    {mode === 'submit' ? 'Submit' : 'Run'}: {result.passed}/{result.total} passed
                  </p>
                  {result.results.map((r, i) => (
                    <details key={i} open={!r.passed} className={r.passed ? 'pass' : 'fail'}>
                      <summary>
                        {r.passed ? '✓' : '✗'} {r.hidden ? 'Hidden test' : 'Test'} {i + 1}
                        {!r.passed && ` — ${r.status}`} <span className="muted">{r.ms}ms</span>
                      </summary>
                      {!r.hidden && <pre>Input:{'\n'}{r.input}</pre>}
                      {!r.hidden && <pre>Expected:{'\n'}{r.expected}</pre>}
                      {!r.passed && <pre>Actual:{'\n'}{r.actual}</pre>}
                      {r.stderr && <pre className="err">{r.stderr}</pre>}
                    </details>
                  ))}
                  {mode === 'submit' && (
                    <div className="review">
                      <button onClick={askReview} disabled={reviewing}>
                        {reviewing ? 'Claude is reviewing…' : review ? 'Review again' : 'Review with Claude'}
                      </button>
                      {reviewErr && <pre className="err">{reviewErr}</pre>}
                      {review && <div className="md"><Markdown>{review}</Markdown></div>}
                    </div>
                  )}
                </>
              )}
            </div>
          </section>
        </>
      ) : <div className="empty muted">No problems yet. Add JSON files to <code>problems/</code>.</div>}
      </main>
    </div>
  )
}
