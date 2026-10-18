import { useEffect, useRef, useState } from 'react'

export type Summary = { slug: string; title: string; difficulty: string; solved: boolean; custom?: boolean; jd?: string | null }

// A dropdown that marks solved problems and shows overall progress.
export default function ProblemPicker({ list, value, onChange, onDelete }: {
  list: Summary[]; value: string | null; onChange: (slug: string) => void; onDelete: (p: Summary) => void
}) {
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  const current = list.find(p => p.slug === value)
  const solved = list.filter(p => p.solved).length

  useEffect(() => {
    if (!open) return
    const away = (e: MouseEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false) }
    const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false) }
    document.addEventListener('mousedown', away)
    document.addEventListener('keydown', esc)
    return () => { document.removeEventListener('mousedown', away); document.removeEventListener('keydown', esc) }
  }, [open])

  const mark = (p: Summary) => (
    <span className={'mark' + (p.solved ? ' done' : '')} title={p.solved ? 'Solved' : 'Not solved yet'}>
      {p.solved ? '✓' : ''}
    </span>
  )
  const tag = (p: Summary) => <span className={'diff ' + p.difficulty.toLowerCase()}>{p.difficulty}</span>
  const jd = (p: Summary) => p.custom && <span className="jd-tag" title="Generated from a job description">JD</span>

  return (
    <div className="pp" ref={root}>
      <button className="pp-btn" onClick={() => setOpen(o => !o)} aria-haspopup="listbox" aria-expanded={open}>
        {current ? <>{mark(current)}<span className="pp-title">{current.title}</span>{jd(current)}{tag(current)}</> : <span className="muted">No problems yet</span>}
        <span className="pp-caret">▾</span>
      </button>
      {open && (
        <div className="pp-menu" role="listbox">
          <div className="pp-head">{solved}/{list.length} solved</div>
          {list.map(p => (
            <div key={p.slug} className="pp-row">
              <button
                role="option" aria-selected={p.slug === value}
                className={'pp-item' + (p.slug === value ? ' active' : '')}
                onClick={() => { onChange(p.slug); setOpen(false) }}
              >
                {mark(p)}<span className="pp-title">{p.title}</span>{jd(p)}{tag(p)}
              </button>
              {p.custom && (
                <button className="pp-del ghost" title="Delete this generated problem" aria-label={`Delete ${p.title}`}
                  onClick={() => { setOpen(false); onDelete(p) }}>✕</button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
