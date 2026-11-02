import { ref } from 'vue'
import { cancelInstall, install, message, type InstallProgress } from './api'
export interface DownloadTask {
	id: string
	instanceId: string
	projectId: number
	fileId?: number
	title: string
	icon?: string
	status: 'running' | 'canceling' | 'finished' | 'failed' | 'canceled'
	progress: InstallProgress
	error?: string
}
const completions = new Map<string, Promise<string[]>>()
export function waitForDownload(task: DownloadTask) {
	return completions.get(task.id)!
}
export const downloadTasks = ref<DownloadTask[]>([])
export function startDownload(
	input: Pick<DownloadTask, 'instanceId' | 'projectId' | 'fileId' | 'title' | 'icon'>,
	onComplete?: () => void,
): DownloadTask {
	const active = downloadTasks.value.find(
		(job) => job.instanceId === input.instanceId && ['running', 'canceling'].includes(job.status),
	)
	if (active) throw new Error('这个游戏版本已有正在安装的 CurseForge 任务，请等待完成。')
	const id = crypto.randomUUID()
	downloadTasks.value.push({
		instanceId: input.instanceId,
		projectId: input.projectId,
		fileId: input.fileId,
		title: input.title,
		icon: input.icon,
		id,
		status: 'running',
		progress: { phase: '解析依赖', name: input.title, completed: 0, total: 0 },
	})
	const task = downloadTasks.value.find((job) => job.id === id)!
	const completion = install(id, input.instanceId, input.projectId, input.fileId, (value) => {
		task.progress = value
	})
		.then((paths) => {
			task.status = 'finished'
			task.progress = { phase: '安装完成', name: task.title, completed: 1, total: 1 }
			onComplete?.()
			return paths
		})
		.catch((error) => {
			task.error = message(error)
			task.status = task.error.includes('取消') ? 'canceled' : 'failed'
			throw error
		})
	completions.set(task.id, completion)
	void completion.catch(() => {})
	return task
}
export async function stopDownload(task: DownloadTask) {
	if (task.status !== 'running') return
	task.status = 'canceling'
	try {
		await cancelInstall(task.id)
	} catch (e) {
		task.status = 'running'
		task.error = message(e)
	}
}
export function dismissDownload(task: DownloadTask) {
	if (['running', 'canceling'].includes(task.status)) return
	completions.delete(task.id)
	downloadTasks.value = downloadTasks.value.filter((job) => job.id !== task.id)
}
