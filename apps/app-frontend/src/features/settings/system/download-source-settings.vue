<script setup lang="ts">
import { injectNotificationManager } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { getDownloadSources, setDownloadSources } from '@/helpers/download-sources'
import { toError } from '@/helpers/errors'
import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'
const { handleError } = injectNotificationManager()
const modal = injectAppSettingsModal()
const original = ref(await getDownloadSources())
const current = ref({ ...original.value })
const saving = ref(false)
const dirty = computed(() => JSON.stringify(current.value) !== JSON.stringify(original.value))
const automatic = computed(() =>
	['official_preferred', 'mirror_preferred'].includes(current.value.policy),
)
const errorMessage = ref('')
function reset() {
	current.value = { ...original.value }
	errorMessage.value = ''
}
async function save() {
	if (saving.value) return
	errorMessage.value = ''
	if (
		!Number.isInteger(current.value.slow_kib_per_second) ||
		current.value.slow_kib_per_second < 16 ||
		current.value.slow_kib_per_second > 8192 ||
		!Number.isInteger(current.value.observation_seconds) ||
		current.value.observation_seconds < 3 ||
		current.value.observation_seconds > 60
	) {
		errorMessage.value = '阈值须为 16–8192 KiB/s，观察时间须为 3–60 秒。'
		return
	}
	saving.value = true
	try {
		await setDownloadSources({ ...current.value })
		original.value = { ...current.value }
	} catch (error) {
		handleError(toError(error))
		errorMessage.value = '保存失败，请重试。'
	} finally {
		saving.value = false
	}
}
onMounted(() =>
	modal.registerUnsavedChangesController({
		getOriginal: () => ({ ...original.value }),
		getModified: () => ({ ...current.value }),
		isSaving: () => saving.value,
		hasChanges: () => dirty.value,
		reset,
		save,
	}),
)
onBeforeUnmount(() => modal.registerUnsavedChangesController(null))
</script>
<template>
	<div class="flex flex-col gap-5">
		<div class="flex flex-col gap-2">
			<label for="gecl-download-policy" class="font-semibold text-contrast">下载策略</label>
			<select id="gecl-download-policy" v-model="current.policy" class="source-control">
				<option value="official_only">仅官方源</option>
				<option value="official_preferred">优先官方，慢速或失败时切换镜像</option>
				<option value="mirror_preferred">优先国内镜像，失败时切回官方</option>
				<option value="mirror_only">仅国内镜像（支持的文件）</option>
			</select>
		</div>
		<div v-if="automatic" class="flex flex-wrap items-end gap-4">
			<div class="flex flex-col gap-2">
				<label for="gecl-download-speed">慢速阈值（KiB/s）</label
				><input
					id="gecl-download-speed"
					v-model.number="current.slow_kib_per_second"
					type="number"
					min="16"
					max="8192"
					step="16"
					class="source-control w-36"
				/>
			</div>
			<div class="flex flex-col gap-2">
				<label for="gecl-download-observation">观察时间（秒）</label
				><input
					id="gecl-download-observation"
					v-model.number="current.observation_seconds"
					type="number"
					min="3"
					max="60"
					step="1"
					class="source-control w-32"
				/>
			</div>
		</div>
		<p class="m-0 text-secondary leading-relaxed">
			国内镜像由
			<a
				href="https://bmclapidoc.bangbang93.com/"
				target="_blank"
				rel="noopener noreferrer"
				class="text-blue"
				>BMCLAPI</a
			>
			提供，支持 Minecraft、游戏资源和部分加载器。Mod 文件仍从原来源下载。
		</p>
		<p v-if="automatic" class="m-0 text-secondary">
			持续低于阈值或下载失败时切换备用源。设置从下一次下载起生效。
		</p>
		<p v-if="errorMessage" role="alert" class="m-0 text-red">{{ errorMessage }}</p>
	</div>
</template>
<style scoped>
.source-control {
	box-sizing: border-box;
	padding: 9px 12px;
	border: 1px solid var(--color-divider);
	border-radius: 8px;
	background: var(--color-raised-bg);
	color: var(--color-contrast);
	font: inherit;
}
.source-control:focus-visible {
	outline: 2px solid var(--color-brand);
	outline-offset: 2px;
}
</style>
