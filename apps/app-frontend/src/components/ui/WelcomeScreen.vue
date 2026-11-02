<script setup lang="ts">
import { BoxImportIcon, PlusIcon, UploadIcon } from '@modrinth/assets'
import { Button } from '@modrinth/ui'
import { inject, onMounted, onUnmounted, ref } from 'vue'
const showCreationModal = inject<(mode?: 'custom') => void>('showCreationModal')
const showFolderImport = inject<() => void>('showFolderImport')
const showImportModal = inject<() => void>('showImportModal')
const showModpackImportModal = inject<() => void>('showModpackImportModal')
const offline = ref(!navigator.onLine)
function updateConnection() {
	offline.value = !navigator.onLine
}
function onKey(event: KeyboardEvent) {
	const target = event.target as HTMLElement | null
	if (
		event.key.toLowerCase() !== 'n' ||
		event.repeat ||
		event.metaKey ||
		event.ctrlKey ||
		event.altKey ||
		target?.isContentEditable ||
		['INPUT', 'TEXTAREA', 'SELECT'].includes(target?.tagName ?? '') ||
		document.querySelector('dialog[open], [role="dialog"]')
	)
		return
	if (!offline.value) {
		event.preventDefault()
		showCreationModal?.('custom')
	}
}
onMounted(() => {
	window.addEventListener('online', updateConnection)
	window.addEventListener('offline', updateConnection)
	window.addEventListener('keydown', onKey)
})
onUnmounted(() => {
	window.removeEventListener('online', updateConnection)
	window.removeEventListener('offline', updateConnection)
	window.removeEventListener('keydown', onKey)
})
</script>
<template>
	<section
		class="flex min-h-full items-center justify-center p-6 box-border"
		aria-label="添加游戏版本"
	>
		<div class="flex flex-col items-center gap-5 w-full max-w-md">
			<img src="/gecl.svg" alt="" class="size-16" />
			<h1 class="m-0 text-2xl font-semibold text-contrast">还没有游戏版本</h1>
			<div class="flex w-full max-w-72 flex-col gap-3">
				<Button
					type="colored"
					color="brand"
					size="lg"
					:disabled="offline"
					@click="showCreationModal?.('custom')"
					><PlusIcon />创建游戏版本</Button
				>
				<Button size="lg" @click="showModpackImportModal?.()"><UploadIcon />导入整合包</Button>
				<Button size="lg" @click="showFolderImport?.()"><BoxImportIcon />导入已有游戏</Button>
			</div>
			<Button type="transparent" @click="showImportModal?.()">从其他启动器导入</Button>
			<p v-if="offline" class="m-0 text-sm text-secondary">当前未联网，下载新版本需要连接网络。</p>
		</div>
	</section>
</template>
