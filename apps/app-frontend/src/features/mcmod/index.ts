import { fetch } from '@tauri-apps/plugin-http'
import {
	displayModName,
	linkedEntry,
	linkedToProject,
	mcmodUrl,
	lookupQueries,
	searchCandidates,
	verifiedEntry,
	type LinkedMcmodEntry,
	type McmodEntry,
	type McmodProject,
} from './parser'
export { displayModName, mcmodUrl }
export type { McmodEntry, McmodProject }
type Cached = { slug?: string | null; entry: McmodEntry | null; expires: number }
const cache = new Map<string, Cached>()
type LookupResult = { entry: McmodEntry | null; failed: boolean }
const pending = new Map<string, Promise<LookupResult>>()
const STORAGE_KEY = 'gecl-mcmod-v2'
try {
	const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
	for (const [key, value] of Object.entries(stored) as [string, Cached][]) {
		if (!value || !value.entry || !Number.isFinite(value.expires) || value.expires <= Date.now())
			continue
		const entry = value.entry
		if (
			entry &&
			(!Number.isSafeInteger(entry.classId) ||
				entry.classId <= 0 ||
				typeof entry.name !== 'string' ||
				entry.name.length > 120)
		)
			continue
		cache.set(key, value)
	}
} catch {
	/* A malformed optional cache never prevents resource browsing. */
}
const LINKS_KEY = 'gecl-mcmod-links-v1'
const LINKS_LIMIT = 3000
type Learned = { entry: LinkedMcmodEntry; expires: number }
const learned = new Map<number, Learned>()
const linkIndex = new Map<string, number>()
const linkKeys = (entry: LinkedMcmodEntry) => [
	...entry.identifiers.map((id) => `mr:${id}`),
	...(entry.curseforgeSlugs ?? []).map((slug) => `cf:${slug}`),
	...(entry.curseforgeIds ?? []).map((id) => `cfid:${id}`),
]
const validLinked = (entry: LinkedMcmodEntry) =>
	Number.isSafeInteger(entry?.classId) &&
	entry.classId > 0 &&
	typeof entry.name === 'string' &&
	entry.name.length <= 120 &&
	typeof entry.originalName === 'string' &&
	[entry.identifiers, entry.curseforgeSlugs ?? [], entry.curseforgeIds ?? []].every(
		(values) =>
			Array.isArray(values) &&
			values.every((id) => typeof id === 'string' && /^[A-Za-z0-9_-]{1,128}$/.test(id)),
	)
function indexLearned(value: Learned) {
	for (const key of linkKeys(value.entry)) linkIndex.set(key, value.entry.classId)
}
try {
	const stored = JSON.parse(localStorage.getItem(LINKS_KEY) ?? '[]') as Learned[]
	for (const value of Array.isArray(stored) ? stored : []) {
		if (!value || !(value.expires > Date.now()) || !validLinked(value.entry)) continue
		learned.set(value.entry.classId, value)
		indexLearned(value)
	}
} catch {
	/* The learned link cache is optional. */
}
let persistTimer: ReturnType<typeof setTimeout> | undefined
/** Remembers an entry page's verified platform links so later lookups need no network request. */
export function learnMcmod(entry: LinkedMcmodEntry) {
	if (!validLinked(entry)) return
	const previous = learned.get(entry.classId)
	if (previous) for (const key of linkKeys(previous.entry)) linkIndex.delete(key)
	learned.delete(entry.classId)
	const value = { entry, expires: Date.now() + 7 * 86400_000 }
	learned.set(entry.classId, value)
	indexLearned(value)
	while (learned.size > LINKS_LIMIT) {
		const oldest = learned.values().next().value!
		learned.delete(oldest.entry.classId)
		for (const key of linkKeys(oldest.entry))
			if (linkIndex.get(key) === oldest.entry.classId) linkIndex.delete(key)
	}
	clearTimeout(persistTimer)
	persistTimer = setTimeout(() => {
		try {
			localStorage.setItem(LINKS_KEY, JSON.stringify([...learned.values()]))
		} catch {
			/* Optional cache. */
		}
	}, 500)
}
export function learnedMcmodClass(classId: number): LinkedMcmodEntry | null {
	const value = learned.get(classId)
	return value && value.expires > Date.now() ? value.entry : null
}
function learnedFor(project: McmodProject): McmodEntry | null {
	const keys =
		project.platform === 'curseforge'
			? [project.slug && `cf:${project.slug.toLowerCase()}`, project.id && `cfid:${project.id}`]
			: [project.id, project.project_id, project.slug].map((id) => id && `mr:${id}`)
	for (const key of keys) {
		const classId = key ? linkIndex.get(key) : undefined
		const linked = classId ? learnedMcmodClass(classId) : null
		const entry = linked ? verifiedEntry(linked, project) : null
		if (entry) return entry
	}
	return null
}
const keyFor = (project: McmodProject) => {
	const id = project.project_id ?? project.id ?? project.slug
	return id ? `${project.platform ?? 'modrinth'}:${id}` : ''
}
export function cachedMcmodEntry(project: McmodProject): McmodEntry | null {
	const cached = cache.get(keyFor(project))
	if (cached?.entry && cached.expires > Date.now() && cached.slug === project.slug)
		return cached.entry
	return learnedFor(project)
}
/** `foreground` is for what the user is waiting on (search, a click); `background` fills in card names. */
export type McmodPriority = 'foreground' | 'background'
type Slot = { priority: number; start: () => void; signal?: AbortSignal }
/** A rate-limited request lane; foreground slots are granted before background ones. */
function createLane(spacing: number) {
	const slots: Slot[] = []
	let lastStart = 0
	let pumping = false
	async function pump() {
		if (pumping) return
		pumping = true
		try {
			while (slots.length) {
				const wait = lastStart + spacing - Date.now()
				if (wait > 0) await new Promise((resolve) => setTimeout(resolve, wait))
				const index = slots.reduce(
					(best, slot, i) => (slot.priority < slots[best].priority ? i : best),
					0,
				)
				const [slot] = slots.splice(index, 1)
				if (!slot || slot.signal?.aborted) continue
				lastStart = Date.now()
				slot.start()
			}
		} finally {
			pumping = false
		}
	}
	return (priority: McmodPriority, signal?: AbortSignal): Promise<void> => {
		signal?.throwIfAborted()
		return new Promise((resolve, reject) => {
			const slot: Slot = { priority: priority === 'foreground' ? 0 : 1, start: resolve, signal }
			slots.push(slot)
			// Cancelled requests leave the queue at once instead of holding a rate-limit slot.
			signal?.addEventListener(
				'abort',
				() => {
					const index = slots.indexOf(slot)
					if (index >= 0) slots.splice(index, 1)
					reject(signal.reason ?? new DOMException('已取消', 'AbortError'))
				},
				{ once: true },
			)
			void pump()
		})
	}
}
// MC百科 answers search bursts faster than about one per second with a "搜索太频繁" page; entry pages tolerate four per second.
const searchLane = createLane(1000)
const pageLane = createLane(250)
const SEARCH_THROTTLED = '搜索太频繁'
export async function publicHtml(
	url: string,
	signal?: AbortSignal,
	priority: McmodPriority = 'background',
): Promise<string> {
	const search = new URL(url).hostname === 'search.mcmod.cn'
	for (let attempt = 0; ; attempt++) {
		const text = await fetchPublicHtml(url, signal, priority, search)
		if (!search || !text.includes(SEARCH_THROTTLED)) return text
		// A throttled page is not a "no results" answer, so it must never be cached as a miss.
		if (attempt >= 2) throw new Error('MC百科搜索太频繁，请稍后重试')
		await new Promise((resolve) => setTimeout(resolve, 1500))
		signal?.throwIfAborted()
	}
}
async function fetchPublicHtml(
	url: string,
	signal: AbortSignal | undefined,
	priority: McmodPriority,
	search: boolean,
): Promise<string> {
	await (search ? searchLane : pageLane)(priority, signal)
	signal?.throwIfAborted()
	const controller = new AbortController()
	const abort = () => controller.abort(signal?.reason)
	signal?.addEventListener('abort', abort, { once: true })
	const timer = setTimeout(() => controller.abort(), 10_000)
	try {
		const response = await fetch(url, {
			method: 'GET',
			signal: controller.signal,
			connectTimeout: 5_000,
		})
		if (!response.ok) throw new Error(`MC百科 HTTP ${response.status}`)
		const text = await response.text()
		if (text.length > 2_000_000) throw new Error('MC百科页面过大')
		return text
	} finally {
		clearTimeout(timer)
		signal?.removeEventListener('abort', abort)
	}
}
let cacheTimer: ReturnType<typeof setTimeout> | undefined
function persistCache() {
	while (cache.size > 2000) cache.delete(cache.keys().next().value!)
	clearTimeout(cacheTimer)
	cacheTimer = setTimeout(() => {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(cache)))
		} catch {
			/* Optional cache. */
		}
	}, 500)
}
export function rememberMcmod(project: McmodProject, entry: McmodEntry) {
	cache.set(keyFor(project), { slug: project.slug, entry, expires: Date.now() + 7 * 86400_000 })
	persistCache()
}
export async function resolveMcmod(
	project: McmodProject,
	signal?: AbortSignal,
	options: { retryMissing?: boolean; throwOnError?: boolean; priority?: McmodPriority } = {},
): Promise<McmodEntry | null> {
	const key = keyFor(project)
	if (!key || typeof project.title !== 'string' || !project.title.trim()) return null
	const known = learnedFor(project)
	if (known) return known
	const cached = cache.get(key)
	if (
		cached &&
		cached.expires > Date.now() &&
		cached.slug === project.slug &&
		(cached.entry || !options.retryMissing)
	)
		return cached.entry
	const priority = options.priority ?? (options.retryMissing ? 'foreground' : 'background')
	let work = pending.get(key)
	if (!work) {
		work = (async (): Promise<LookupResult> => {
			const controller = new AbortController()
			const abort = () => controller.abort()
			signal?.addEventListener('abort', abort, { once: true })
			if (signal?.aborted) abort()
			const timer = setTimeout(abort, 20_000)
			let entry: McmodEntry | null = null
			let failed = false
			const visited = new Set<number>()
			try {
				for (const title of lookupQueries(project)) {
					if (controller.signal.aborted) {
						failed = true
						break
					}
					try {
						const query = new URL('https://search.mcmod.cn/s')
						query.searchParams.set('key', title)
						query.searchParams.set('mold', '1')
						const html = await publicHtml(query.toString(), controller.signal, priority)
						for (const classId of searchCandidates(html, title)) {
							if (controller.signal.aborted) {
								failed = true
								break
							}
							if (visited.has(classId)) continue
							visited.add(classId)
							try {
								let linked = learnedMcmodClass(classId)
								if (!linked) {
									linked = linkedEntry(
										await publicHtml(
											`https://www.mcmod.cn/class/${classId}.html`,
											controller.signal,
											priority,
										),
										classId,
									)
									if (linked) learnMcmod(linked)
								}
								entry = verifiedEntry(linked, project)
								if (entry) break
							} catch {
								failed = true
							}
						}
					} catch {
						failed = true
					}
					if (entry) break
				}
				if (controller.signal.aborted) return { entry: null, failed: true }
				if (entry || !failed) {
					cache.set(key, {
						slug: project.slug,
						entry,
						expires: Date.now() + (entry ? 7 * 86400_000 : 600_000),
					})
					persistCache()
				}
				return { entry, failed }
			} finally {
				clearTimeout(timer)
				signal?.removeEventListener('abort', abort)
			}
		})()
		pending.set(key, work)
	}
	try {
		const result = await work
		if (signal?.aborted) return null
		if (!result.entry && result.failed && options.throwOnError)
			throw new Error('暂时无法查询 MC 百科，请重试。')
		return result.entry
	} finally {
		if (pending.get(key) === work) pending.delete(key)
	}
}
