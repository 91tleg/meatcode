import { useEffect, useRef, useState } from 'react'

export type Summary = { slug: string; title: string; difficulty: string; solved: boolean }

// A dropdown that marks solved problems and shows overall progress.
export default function ProblemPicker({ list, value, onChange }: {
  list: Summary[]; value: string | null; onChange: (slug: string) => void
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

  return (
    <div className="pp" ref={root}>
      <button className="pp-btn" onClick={() => setOpen(o => !o)} aria-haspopup="listbox" aria-expanded={open}>
        {current ? <>{mark(current)}<span className="pp-title">{current.title}</span>{tag(current)}</> : <span className="muted">No problems yet</span>}
        <span className="pp-caret">▾</span>
      </button>
      {open && (
        <div className="pp-menu" role="listbox">
          <div className="pp-head">{solved}/{list.length} solved</div>
          {list.map(p => (
            <button
              key={p.slug} role="option" aria-selected={p.slug === value}
              className={'pp-item' + (p.slug === value ? ' active' : '')}
              onClick={() => { onChange(p.slug); setOpen(false) }}
            >
              {mark(p)}<span className="pp-title">{p.title}</span>{tag(p)}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
