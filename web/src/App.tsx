import { useEffect, useRef, useState } from 'react'
import Editor, { DiffEditor } from '@monaco-editor/react'
import Markdown from 'react-markdown'
import ProblemPicker, { type Summary } from './ProblemPicker'
import CheatSheet from './CheatSheet'

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
type RunResult = {
  compile_error: string | null; results: TestResult[]; passed: number; total: number
  submission_id?: number | null
}
type Followup = { question: string; hint: string }
type HistoryItem = {
  id: number; language: string; passed: number; total: number; created_at: number
  has_review: boolean; parent_id: number | null
}
// A follow-up round: edit the first attempt in place to satisfy a new constraint.
type ActiveFollowup = Followup & { parentId: number; baseCode: string }

const ago = (unix: number) => {
  const d = Math.max(0, Date.now() / 1000 - unix)
  if (d < 60) return 'just now'
  if (d < 3600) return `${Math.floor(d / 60)}m ago`
  if (d < 86400) return `${Math.floor(d / 3600)}h ago`
  return `${Math.floor(d / 86400)}d ago`
}
const put = (url: string, body: unknown) =>
  fetch(url, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }).catch(() => {})

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
  const workRef = useRef<HTMLElement>(null)
  const [outH, setOutH] = useState(() => {
    try { return Number(localStorage.getItem('outH')) || 260 } catch { return 260 }
  })
  const [mode, setMode] = useState<'run' | 'submit'>('run')
  const [submissionId, setSubmissionId] = useState<number | null>(null)
  const [history, setHistory] = useState<HistoryItem[]>([])
  const [proposed, setProposed] = useState<Followup | null>(null)
  const [active, setActive] = useState<ActiveFollowup | null>(null)
  const [diff, setDiff] = useState(false)
  const [sheetOpen, setSheetOpen] = useState(false)
  const [draftVersion, setDraftVersion] = useState(0)
  const drafts = useRef<Record<string, string>>({})
  const saveTimers = useRef<Record<string, number>>({})
  const [review, setReview] = useState<string | null>(null)
  const [reviewing, setReviewing] = useState(false)
  const [reviewErr, setReviewErr] = useState<string | null>(null)

  const refreshList = () =>
    fetch('/api/problems').then(r => r.json()).then((l: Summary[]) => { setList(l); return l }).catch(() => [] as Summary[])

  useEffect(() => {
    refreshList().then(l => { if (l.length) setSlug(l[0].slug) })
  }, [])

  const refreshHistory = (sl: string) =>
    fetch(`/api/problems/${sl}/submissions`).then(r => r.json()).then(setHistory).catch(() => {})

  // Load the problem, its saved drafts and its past attempts together.
  useEffect(() => {
    if (!slug) return
    let stale = false
    setResult(null); setSubmissionId(null); setReview(null); setReviewErr(null); setHistory([])
    setProposed(null); setActive(null); setDiff(false)
    Promise.all([
      fetch(`/api/problems/${slug}`).then(r => r.json()),
      fetch(`/api/problems/${slug}/drafts`).then(r => r.json()).catch(() => ({})),
    ]).then(([prob, d]) => {
      if (stale) return
      drafts.current = d
      setProblem(prob)
      setDraftVersion(v => v + 1)
    })
    refreshHistory(slug)
    return () => { stale = true }
  }, [slug])

  // Show the saved draft for this problem+language, else the starter.
  useEffect(() => {
    if (!problem) return
    setCode(drafts.current[lang] ?? problem.starter[lang] ?? '')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [problem, lang, draftVersion])

  // Autosave drafts to the database shortly after the last keystroke.
  const saveDraft = (sl: string, lg: string, text: string) => {
    drafts.current[lg] = text
    window.clearTimeout(saveTimers.current[lg])
    saveTimers.current[lg] = window.setTimeout(() => put(`/api/problems/${sl}/draft/${lg}`, { code: text }), 600)
  }

  const onCode = (v?: string) => {
    const text = v ?? ''
    setCode(text)
    if (problem) saveDraft(problem.slug, lang, text)
  }

  const reset = () => {
    if (!problem) return
    window.clearTimeout(saveTimers.current[lang])
    delete drafts.current[lang]
    fetch(`/api/problems/${problem.slug}/draft/${lang}`, { method: 'DELETE' }).catch(() => {})
    setCode(problem.starter[lang] ?? '')
  }

  // Load a past attempt into the editor and show its saved results and review.
  const openAttempt = async (id: number) => {
    if (!problem) return
    const s = await fetch(`/api/submissions/${id}`).then(r => r.json())
    saveDraft(problem.slug, s.language, s.code)
    setLang(s.language); setCode(s.code); setDiff(false)
    setMode('submit'); setResult(s.results); setSubmissionId(s.id)
    setReview(s.review); setReviewErr(null); setProposed(s.proposed_followup)
    if (s.parent_id && s.followup) {
      const base = await fetch(`/api/submissions/${s.parent_id}`).then(r => r.json())
      setActive({ ...s.followup, parentId: s.parent_id, baseCode: base.code })
    } else setActive(null)
  }

  // Start the follow-up round: reload the reviewed solution so it can be edited in place.
  const startFollowup = async () => {
    if (!problem || !proposed || !submissionId) return
    const base = await fetch(`/api/submissions/${submissionId}`).then(r => r.json())
    saveDraft(problem.slug, base.language, base.code)
    setLang(base.language); setCode(base.code)
    setActive({ ...proposed, parentId: base.id, baseCode: base.code })
    setProposed(null); setResult(null); setReview(null); setReviewErr(null)
    setSubmissionId(null); setMode('run'); setDiff(false)
  }

  const cancelFollowup = () => { setActive(null); setDiff(false); setResult(null); setSubmissionId(null); setReview(null) }

  const run = async (submit: boolean) => {
    if (!problem) return
    setBusy(true); setMode(submit ? 'submit' : 'run'); setResult(null)
    setReview(null); setReviewErr(null); setSubmissionId(null); setProposed(null)
    try {
      const r = await fetch(`/api/problems/${problem.slug}/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ language: lang, code, submit, parent_id: submit ? active?.parentId : undefined }),
      })
      const res: RunResult = await r.json()
      setResult(res)
      if (submit && res.submission_id) {
        setSubmissionId(res.submission_id)
        refreshHistory(problem.slug)
        if (res.passed === res.total) refreshList() // may have just been solved
      }
    } finally { setBusy(false) }
  }

  // Drag the divider up/down to resize the output panel.
  const startDrag = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault()
    const el = e.currentTarget
    el.setPointerCapture(e.pointerId)
    const startY = e.clientY, startH = outH
    const max = () => (workRef.current?.clientHeight ?? 800) - 120
    const move = (ev: PointerEvent) =>
      setOutH(Math.max(48, Math.min(max(), startH + (startY - ev.clientY))))
    const up = () => {
      el.removeEventListener('pointermove', move)
      el.removeEventListener('pointerup', up)
      setOutH(h => { try { localStorage.setItem('outH', String(h)) } catch {} return h })
    }
    el.addEventListener('pointermove', move)
    el.addEventListener('pointerup', up)
  }

  const askReview = async (force: boolean) => {
    if (!problem || !submissionId) return
    setReviewing(true); setReview(null); setReviewErr(null)
    try {
      const r = await fetch('/api/review', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ submission_id: submissionId, force }),
      })
      if (r.ok) {
        const out = await r.json()
        setReview(out.review); setProposed(out.followup ?? null); refreshHistory(problem.slug)
      } else setReviewErr(await r.text())
    } catch (e) { setReviewErr(String(e)) }
    finally { setReviewing(false) }
  }

  return (
    <div className="app">
      <header className="topbar">
        <h1>meatcode</h1>
        <ProblemPicker list={list} value={slug} onChange={setSlug} />
        <span className="spacer" />
        <button onClick={() => setSheetOpen(o => !o)} className={sheetOpen ? 'primary' : ''}>Cheatsheet</button>
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

          <section className="work" ref={workRef}>
            <div className="toolbar">
              <select value={lang} onChange={e => { setLang(e.target.value); localStorage.setItem('lang', e.target.value) }}>
                {LANGS.map(l => <option key={l.id} value={l.id}>{l.label}</option>)}
              </select>
              <button onClick={reset} className="ghost">Reset</button>
              <select value="" onChange={e => e.target.value && openAttempt(Number(e.target.value))} disabled={history.length === 0}>
                <option value="">History ({history.length})</option>
                {history.map(h => (
                  <option key={h.id} value={h.id}>
                    {h.parent_id ? '↳ follow-up ' : ''}{h.passed === h.total ? '✓' : '✗'} {h.passed}/{h.total} · {LANGS.find(l => l.id === h.language)?.label ?? h.language} · {ago(h.created_at)}{h.has_review ? ' · reviewed' : ''}
                  </option>
                ))}
              </select>
              <span className="spacer" />
              <button disabled={busy} onClick={() => run(false)}>Run</button>
              <button disabled={busy} onClick={() => run(true)} className="primary">Submit</button>
            </div>
            {active && (
              <div className="banner">
                <div><b>Follow-up:</b> {active.question}</div>
                <details><summary>Hint</summary>{active.hint}</details>
                <div className="banner-actions">
                  <button className="ghost" onClick={() => setDiff(d => !d)}>{diff ? 'Back to editing' : 'Compare with first attempt'}</button>
                  <button className="ghost" onClick={cancelFollowup}>Cancel follow-up</button>
                </div>
              </div>
            )}
            <div className="editor">
              {active && diff ? (
                <DiffEditor
                  height="100%" theme="vs-dark" original={active.baseCode} modified={code}
                  language={LANGS.find(l => l.id === lang)?.monaco}
                  options={{ readOnly: true, minimap: { enabled: false }, fontSize: 14, automaticLayout: true, renderSideBySide: true }}
                />
              ) : (
                <Editor
                  height="100%" theme="vs-dark" value={code} onChange={onCode}
                  language={LANGS.find(l => l.id === lang)?.monaco}
                  options={{ minimap: { enabled: false }, fontSize: 14, automaticLayout: true }}
                />
              )}
            </div>
            <div className="divider" onPointerDown={startDrag} title="Drag to resize" />
            <div className="results" style={{ height: outH }}>
              {busy && <p className="muted">Running…</p>}
              {result?.compile_error && <pre className="err">{result.compile_error}</pre>}
              {result && !result.compile_error && (
                <>
                  <p className={result.passed === result.total ? 'ok' : 'bad'}>
                    {mode === 'submit' ? (active ? 'Follow-up submit' : 'Submit') : 'Run'}: {result.passed}/{result.total} passed
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
                </>
              )}
              {result && !busy && mode === 'submit' && submissionId && (
                <div className="review">
                  <button onClick={() => askReview(!!review)} disabled={reviewing}>
                    {reviewing ? 'Claude is reviewing…' : review ? 'Regenerate review' : active ? 'Review follow-up' : 'Review with Claude'}
                  </button>
                  {reviewErr && <pre className="err">{reviewErr}</pre>}
                  {review && <div className="md"><Markdown>{review}</Markdown></div>}
                  {review && proposed && !active && (
                    <div className="followup-card">
                      <b>Follow-up</b>
                      <p>{proposed.question}</p>
                      <details><summary>Hint</summary>{proposed.hint}</details>
                      <button className="primary" onClick={startFollowup}>Work on this follow-up</button>
                    </div>
                  )}
                </div>
              )}
            </div>
          </section>
        </>
      ) : <div className="empty muted">No problems yet. Add JSON files to <code>problems/</code>.</div>}
      </main>
      {sheetOpen && <CheatSheet onClose={() => setSheetOpen(false)} />}
    </div>
  )
}
