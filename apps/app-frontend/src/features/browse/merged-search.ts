export interface SearchChunk<T> {
	items: T[]
	total: number
	consumed?: number
}
export interface SearchFeed<T> {
	name: string
	load: (offset: number, limit: number) => Promise<SearchChunk<T>>
}
interface Stream<T> {
	offset: number
	total: number
	buffer: T[]
	ended: boolean
	seen: Set<string>
	error?: string
}
interface Snapshot<T> {
	created: number
	streams: Stream<T>[]
	items: T[]
	steps: number
	filtered: number
	pending: Promise<unknown>
}

export function createMergedSearch<T>(identity: (item: T) => string) {
	const cache = new Map<string, Snapshot<T>>()
	return {
		clear() {
			cache.clear()
		},
		async page(options: {
			key: string
			offset: number
			limit: number
			feeds: SearchFeed<T>[]
			signal: AbortSignal
			compare?: (a: T, b: T) => number
			include?: (item: T) => boolean
		}) {
			let snapshot = cache.get(options.key)
			if (!snapshot || Date.now() - snapshot.created > 60_000) {
				snapshot = {
					created: Date.now(),
					streams: options.feeds.map(() => ({
						offset: 0,
						total: 0,
						buffer: [],
						ended: false,
						seen: new Set(),
					})),
					items: [],
					steps: 0,
					filtered: 0,
					pending: Promise.resolve(),
				}
				cache.set(options.key, snapshot)
				while (cache.size > 3) cache.delete(cache.keys().next().value!)
			}
			const state = snapshot
			const check = () => {
				if (options.signal.aborted) throw new DOMException('搜索已更新', 'AbortError')
			}
			const run = async () => {
				check()
				const fill = async (stream: Stream<T>, index: number) => {
					while (!stream.buffer.length && !stream.ended) {
						check()
						try {
							const chunk = await options.feeds[index].load(stream.offset, 50)
							check()
							const consumed = chunk.consumed ?? chunk.items.length
							stream.offset += consumed
							stream.total = chunk.total
							stream.ended = consumed === 0 || stream.offset >= chunk.total
							stream.buffer.push(
								...chunk.items.filter((item) => {
									const id = identity(item)
									if (stream.seen.has(id)) return false
									stream.seen.add(id)
									return true
								}),
							)
						} catch (error) {
							check()
							stream.error = `${options.feeds[index].name}：${(error as { message?: string })?.message ?? String(error)}`
							stream.ended = true
						}
					}
				}
				await Promise.all(state.streams.map(fill))
				while (state.items.length < Math.min(10000, options.offset + options.limit)) {
					check()
					await Promise.all(state.streams.map(fill))
					const available = state.streams
						.map((s, i) => ({ s, i }))
						.filter(({ s }) => s.buffer.length)
					if (!available.length) break
					let index = available[0].i
					if (options.compare) {
						index = available.reduce(
							(best, value) =>
								options.compare!(value.s.buffer[0], state.streams[best].buffer[0]) < 0
									? value.i
									: best,
							index,
						)
					} else {
						const preferred = state.steps % state.streams.length
						index = available.find(({ i }) => i >= preferred)?.i ?? available[0].i
					}
					const item = state.streams[index].buffer.shift()!
					state.steps++
					if (!options.include || options.include(item)) state.items.push(item)
					else state.filtered++
				}
				return {
					items: state.items.slice(options.offset, options.offset + options.limit),
					total: state.streams.every((stream) => stream.ended && !stream.buffer.length)
						? state.items.length
						: Math.max(
								state.items.length,
								state.streams.reduce((sum, stream) => sum + stream.total, 0) - state.filtered,
							),
					errors: state.streams.flatMap((stream) => (stream.error ? [stream.error] : [])),
				}
			}
			const result = state.pending.catch(() => {}).then(run)
			state.pending = result
			return result
		},
	}
}
