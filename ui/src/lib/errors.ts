import { m } from '../paraglide/messages'
import { getLocale } from '../paraglide/runtime'

/**
 * Turn a Tauri-side error into a human-readable string.
 *
 * UnkaiError is a Rust enum with `#[derive(Serialize)]`, which serde
 * serialises as an externally-tagged object: `{ "Network": "..." }`,
 * `{ "Auth": "..." }`, etc. The variant name tells us the category,
 * the inner string is the message.
 *
 * Backend strings stay English by convention. For Spanish, the four
 * connection categories a user can actually act on are mapped here
 * to short, actionable messages; anything else falls through
 * unchanged. This is deliberately a presentation-layer mapping, not
 * a rewrite of the Rust error variants.
 */
function translateConnectionError(raw: string): string | null {
  if (!raw || getLocale() !== 'es') return null
  const s = raw.toLowerCase()
  if (/certificate|tls|ssl|handshake/.test(s)) return m.error_connection_tls()
  if (/auth|login|password|credential|\b535\b/.test(s)) return m.error_connection_auth()
  if (/refused|econnrefused/.test(s)) return m.error_connection_refused()
  if (/timed?\s?out|etimedout|timeout/.test(s)) return m.error_connection_timeout()
  if (/dns|no such host|name or service not known|lookup/.test(s)) return m.error_connection_dns()
  if (/network|unreachable|connection|connect/.test(s)) return m.error_connection_generic()
  return null
}

function finish(raw: string): string {
  return translateConnectionError(raw) ?? raw
}

export function formatError(e: unknown): string {
  if (e == null) return ''
  if (typeof e === 'string') return finish(e)
  if (e instanceof Error) return finish(e.message)

  if (typeof e === 'object') {
    const obj = e as Record<string, unknown>
    if (typeof obj.message === 'string') return finish(obj.message)

    // Externally tagged UnkaiError: first key is the variant name,
    // its value is the message string.
    const entries = Object.entries(obj)
    if (entries.length === 1) {
      const [variant, value] = entries[0]
      if (typeof value === 'string') return finish(`${variant}: ${value}`)
    }

    try {
      return finish(JSON.stringify(e))
    } catch {
      return String(e)
    }
  }
  return String(e)
}
