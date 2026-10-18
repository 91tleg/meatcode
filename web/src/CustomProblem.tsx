import { useRef, useState } from 'react'
import type { Summary } from './ProblemPicker'

const MIN = 80
const MAX = 20000

export type JdSummary = { id: string; title: string; problems: number; solved: number }

// Two flows: paste/upload a new job description (creates it plus its first problem), or add another problem to an
// existing one. Either way the server only keeps a problem whose two reference solutions agree on every test.
export default function CustomProblem({ jd, onClose, onCreated }: {
  jd?: JdSummary; onClose: () => void; onCreated: (p: Summary) => void
}) {
  const [text, setText] = useState('')
  const [title, setTitle] = useState('')
  const [difficulty, setDifficulty] = useState('Medium')
  const [focus, setFocus] = useState('')
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  const file = useRef<HTMLInputElement>(null)
  const len = text.trim().length
  const ok = jd ? true : len >= MIN && len <= MAX

  const load = async (f: File | undefined) => {
    if (!f) return
    setErr(null)
    if (!/\.(txt|md|markdown)$/i.test(f.name) && !f.type.startsWith('text/')) {
      setErr('Upload a plain-text or Markdown file, or paste the text. PDF and Word files are not read.')
      return
    }
    setText(await f.text())
    if (!title) setTitle(f.name.replace(/\.[^.]+$/, '').slice(0, 60))
  }

  const generate = async () => {
    setBusy(true); setErr(null)
    try {
      const r = await fetch(jd ? `/api/jds/${jd.id}/problems` : '/api/jds', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(jd ? { difficulty, focus } : { jd: text, title, difficulty, focus }),
      })
      if (!r.ok) { setErr((await r.text()) || `Request failed (${r.status})`); return }
      onCreated(await r.json())
    } catch {
      setErr('Could not reach the API.')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="modal-back" onMouseDown={e => { if (e.target === e.currentTarget && !busy) onClose() }}>
      <div className="modal" role="dialog" aria-label={jd ? 'Add a problem' : 'New job description'}>
        <div className="sheet-head">
          <b>{jd ? `Another problem for "${jd.title}"` : 'New job description'}</b>
          <button className="ghost" onClick={onClose} disabled={busy}>Close</button>
        </div>
        <div className="modal-body">
          <p className="muted">
            {jd
              ? 'Claude picks a different skill from the ones this role already has. '
              : 'Claude picks the skill this role would test and writes a problem. '}
            The tests are not taken on trust: two independent solutions are run on every input and the problem is only kept if they agree.
          </p>
          {!jd && (
            <>
              <div className="modal-row">
                <input className="grow" value={title} onChange={e => setTitle(e.target.value)} disabled={busy} maxLength={60}
                  placeholder="Name (optional, e.g. Stripe – Backend)" />
                <button onClick={() => file.current?.click()} disabled={busy}>Upload .txt / .md</button>
                <input ref={file} type="file" accept=".txt,.md,.markdown,text/plain,text/markdown" hidden
                  onChange={e => { load(e.target.files?.[0]); e.target.value = '' }} />
              </div>
              <textarea
                value={text} onChange={e => setText(e.target.value)} disabled={busy} spellCheck={false}
                placeholder="Paste the job description here…"
              />
            </>
          )}
          <div className="modal-row">
            <select value={difficulty} onChange={e => setDifficulty(e.target.value)} disabled={busy}>
              <option>Easy</option><option>Medium</option><option>Hard</option>
            </select>
            <input className="grow" value={focus} onChange={e => setFocus(e.target.value)} disabled={busy} maxLength={200}
              placeholder="Practice focus (optional, e.g. graphs, sliding window)" />
            {!jd && <span className={'muted' + (len > MAX ? ' bad' : '')}>{len.toLocaleString()} / {MAX.toLocaleString()}</span>}
          </div>
          {err && <pre className="modal-err">{err}</pre>}
          <div className="modal-row">
            <span className="muted">{busy ? 'Generating and verifying. This usually takes under a minute, sometimes a few.' : !jd && len > 0 && len < MIN ? `At least ${MIN} characters.` : ''}</span>
            <span className="spacer" />
            <button className="primary" onClick={generate} disabled={busy || !ok}>{busy ? 'Working…' : 'Generate problem'}</button>
          </div>
        </div>
      </div>
    </div>
  )
}
