import { Channel, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Router } from 'vue-router'

type Request = {
	id: string
	action: string
	payload: { command?: string; args?: Record<string, unknown>; value?: string; selector?: string }
}
export async function setupAgentBridge(router: Router) {
	if (!(await invoke<boolean>('agent_mode'))) return
	await listen<Request>('gecl-agent-request', async ({ payload: request }) => {
		try {
			let value: unknown
			const events: unknown[] = []
			const { action, payload } = request
			if (action === 'commands') value = await invoke('plugin:agent|commands')
			else if (action === 'status')
				value = { ready: true, route: router.currentRoute.value.fullPath }
			else if (action === 'ui.navigate') {
				await router.push(payload.value ?? '/')
				value = router.currentRoute.value.fullPath
			} else if (action === 'ui.snapshot')
				value = {
					route: router.currentRoute.value.fullPath,
					text: document.body.innerText,
					controls: [...document.querySelectorAll('button,input,select,[role="button"]')].map(
						(node) => ({
							tag: node.tagName,
							id: node.id,
							label: node.getAttribute('aria-label') ?? node.textContent?.trim(),
							type: node.getAttribute('type'),
						}),
					),
				}
			else if (action === 'ui.click') {
				const node = document.querySelector<HTMLElement>(payload.value ?? '')
				if (!node) throw Error('找不到控件')
				node.click()
				value = true
			} else if (action === 'ui.fill') {
				const node = document.querySelector<HTMLInputElement | HTMLTextAreaElement>(
					payload.selector ?? '',
				)
				if (!node || !['INPUT', 'TEXTAREA'].includes(node.tagName)) throw Error('找不到输入框')
				const setter = Object.getOwnPropertyDescriptor(
					node.tagName === 'INPUT' ? HTMLInputElement.prototype : HTMLTextAreaElement.prototype,
					'value',
				)?.set
				setter?.call(node, payload.value ?? '')
				node.dispatchEvent(new Event('input', { bubbles: true }))
				node.dispatchEvent(new Event('change', { bubbles: true }))
				value = true
			} else if (action === 'invoke') {
				if (!payload.command || payload.command.startsWith('plugin:agent|'))
					throw Error('无效操作接口')
				const args = { ...(payload.args ?? {}) }
				for (const [key, value] of Object.entries(args))
					if (value === '$channel') {
						const channel = new Channel<unknown>()
						channel.onmessage = (event) => {
							if (events.length < 1000) events.push(event)
						}
						args[key] = channel
					}
				value = await invoke(payload.command, args)
			} else throw Error('未知命令')
			await invoke('plugin:agent|complete', {
				id: request.id,
				result: { ok: true, result: value ?? null, events },
			})
		} catch (error) {
			await invoke('plugin:agent|complete', {
				id: request.id,
				result: { ok: false, error: error instanceof Error ? error.message : error },
			})
		}
	})
	await invoke('plugin:agent|ready')
}
