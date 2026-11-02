<script setup lang="ts">
import { UploadIcon } from '@modrinth/assets'
import { Avatar, Button, injectNotificationManager } from '@modrinth/ui'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ref } from 'vue'

defineProps<{ label?: string }>()
const path = defineModel<string | null>({ default: null })
const busy = ref(false)
const { handleError } = injectNotificationManager()
async function choose() {
	try {
		const source = await open({
			multiple: false,
			filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] }],
		})
		if (typeof source !== 'string') return
		busy.value = true
		path.value = await invoke<string>('plugin:gecl|cache_artwork', { source })
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}
</script>
<template>
	<div class="flex items-center gap-3">
		<Avatar
			:src="path ? convertFileSrc(path) : undefined"
			alt="整合包图标"
			size="3.5rem"
			no-shadow
		/>
		<Button type="outlined" :disabled="busy" @click="choose"
			><UploadIcon />{{ busy ? '处理中…' : (label ?? '选择图标') }}</Button
		>
	</div>
</template>
