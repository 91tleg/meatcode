import { useEffect, useState } from 'react'
import Markdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

type SheetInfo = { slug: string; title: string }

const remember = (key: string, value?: string) => {
  try { if (value === undefined) return localStorage.getItem(key); localStorage.setItem(key, value) } catch { /* ignore */ }
  return null
}

// A slide-in panel with the Markdown cheatsheets from `cheatsheets/`.
export default function CheatSheet({ onClose }: { onClose: () => void }) {
  const [sheets, setSheets] = useState<SheetInfo[]>([])
  const [slug, setSlug] = useState<string | null>(remember('sheet'))
  const [loaded, setLoaded] = useState<{ slug: string; markdown: string } | null>(null)
  const [failed, setFailed] = useState<string | null>(null) // a slug that could not be loaded, or '*' for the list

  useEffect(() => {
    fetch('/api/cheatsheets').then(r => r.json()).then((l: SheetInfo[]) => {
      setSheets(l)
      setSlug(cur => (cur && l.some(s => s.slug === cur) ? cur : l[0]?.slug ?? null))
    }).catch(() => setFailed('*'))
  }, [])

  useEffect(() => {
    if (!slug) return
    remember('sheet', slug)
    fetch(`/api/cheatsheets/${slug}`)
      .then(r => (r.ok ? r.json() : Promise.reject()))
      .then(s => setLoaded({ slug, markdown: s.markdown }))
      .catch(() => setFailed(slug))
  }, [slug])

  const markdown = loaded?.slug === slug ? loaded.markdown : null
  const error = failed === '*' ? 'Could not load the cheatsheets.' : failed === slug ? 'Could not load this sheet.' : null

  useEffect(() => {
    const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose() }
    document.addEventListener('keydown', esc)
    return () => document.removeEventListener('keydown', esc)
  }, [onClose])

  return (
    <aside className="sheet" aria-label="Cheatsheet">
      <div className="sheet-head">
        <b>Cheatsheet</b>
        <button className="ghost" onClick={onClose} aria-label="Close cheatsheet">✕</button>
      </div>
      <div className="sheet-tabs">
        {sheets.length === 0 && !error && <span className="muted">No cheatsheets yet. Add Markdown files to <code>cheatsheets/</code>.</span>}
        {sheets.map(s => (
          <button key={s.slug} className={'tab' + (s.slug === slug ? ' active' : '')} onClick={() => setSlug(s.slug)}>
            {s.title}
          </button>
        ))}
      </div>
      <div className="sheet-body md">
        {error && <p className="err">{error}</p>}
        {!markdown && !error && slug && <p className="muted">Loading…</p>}
        {markdown && <Markdown remarkPlugins={[remarkGfm]}>{markdown}</Markdown>}
      </div>
    </aside>
  )
}
