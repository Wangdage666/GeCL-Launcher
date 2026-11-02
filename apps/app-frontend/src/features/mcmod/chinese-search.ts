import { publicHtml } from './index'
import { linkedEntry, searchCandidates, type LinkedMcmodEntry } from './parser'

const storageKey = 'gecl-chinese-mod-search-v2'
type Cached = { entries: LinkedMcmodEntry[]; expires: number }
const cache = new Map<string, Cached>()
const normalize = (query: string) => query.normalize('NFKC').trim().toLowerCase()
try {
	const stored = JSON.parse(localStorage.getItem(storageKey) ?? '{}')
	for (const [query, value] of Object.entries(stored) as [string, Cached][]) {
		if (
			value.expires > Date.now() &&
			Array.isArray(value.entries) &&
			value.entries.every(
				(entry) =>
					Number.isSafeInteger(entry.classId) &&
					entry.classId > 0 &&
					typeof entry.name === 'string' &&
					typeof entry.originalName === 'string' &&
					Array.isArray(entry.identifiers) &&
					entry.identifiers.every((id) => /^[A-Za-z0-9_-]{1,128}$/.test(id)),
			)
		)
			cache.set(query, value)
	}
} catch {
	/* A damaged optional name cache does not block browsing. */
}

export async function findChineseMods(
	query: string,
	signal: AbortSignal,
): Promise<LinkedMcmodEntry[]> {
	const key = normalize(query)
	const existing = cache.get(key)
	if (existing && existing.expires > Date.now()) return existing.entries
	const url = new URL('https://search.mcmod.cn/s')
	url.searchParams.set('key', query)
	url.searchParams.set('mold', '1')
	const candidates = searchCandidates(await publicHtml(url.toString(), signal), query, 8)
	const entries: (LinkedMcmodEntry | null)[] = Array(candidates.length).fill(null)
	let next = 0
	let failures = 0
	async function worker() {
		while (next < candidates.length && !signal.aborted) {
			const index = next++
			try {
				entries[index] = linkedEntry(
					await publicHtml(`https://www.mcmod.cn/class/${candidates[index]}.html`, signal),
					candidates[index],
				)
			} catch (error) {
				if (signal.aborted) throw error
				failures++
			}
		}
	}
	await Promise.all([worker(), worker()])
	signal.throwIfAborted()
	const matches = entries.filter((entry): entry is LinkedMcmodEntry => entry !== null)
	// Do not cache a transient failure as a definitive missing Chinese name.
	if (failures === 0) {
		cache.set(key, {
			entries: matches,
			expires: Date.now() + (matches.length ? 86400_000 : 600_000),
		})
		while (cache.size > 128) cache.delete(cache.keys().next().value!)
		try {
			localStorage.setItem(storageKey, JSON.stringify(Object.fromEntries(cache)))
		} catch {
			/* Optional cache. */
		}
	}
	if (failures && !matches.length) throw new Error('暂时无法查询中文名称，请稍后重试或使用英文名。')
	return matches
}

export function withChineseMatches(requestParams: string, entries: LinkedMcmodEntry[]): string {
	const identifiers = [...new Set(entries.flatMap((entry) => entry.identifiers))].filter((id) =>
		/^[A-Za-z0-9_-]{1,128}$/.test(id),
	)
	if (!identifiers.length) return requestParams
	const params = new URLSearchParams(requestParams.replace(/^\?/, ''))
	const oldFilter = params.get('new_filters')
	const values = identifiers.map((id) => JSON.stringify(id)).join(', ')
	const mappingFilter = `(slug IN [${values}] OR project_id IN [${values}])`
	params.set('new_filters', oldFilter ? `(${oldFilter}) AND ${mappingFilter}` : mappingFilter)
	params.delete('query')
	return `?${params}`
}
