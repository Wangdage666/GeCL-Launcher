<script setup lang="ts">
import { Button, Input, NewModal } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { onBeforeUnmount, ref } from 'vue'
import ArtworkPicker from './artwork-picker.vue'
const modal = ref<InstanceType<typeof NewModal>>()
const name = ref('')
const icon = ref<string | null>(null)
let resolve: ((value: { name: string; iconPath: string | null } | null) => void) | undefined
function cancel() {
	resolve?.(null)
	resolve = undefined
}
function proceed() {
	resolve?.({ name: name.value.trim(), iconPath: icon.value })
	resolve = undefined
	modal.value?.hide()
}
async function show(source: string) {
	cancel()
	const preview = await invoke<{ name: string; icon: string | null }>('plugin:gecl|pack_artwork', {
		source,
	})
	name.value = preview.name
	icon.value = preview.icon
	modal.value?.show()
	return new Promise<{ name: string; iconPath: string | null } | null>((done) => {
		resolve = done
	})
}
onBeforeUnmount(cancel)
defineExpose({ show })
</script>
<template>
	<NewModal ref="modal" header="导入整合包" width="30rem" :on-hide="cancel">
		<div class="flex flex-col gap-4">
			<ArtworkPicker v-model="icon" />
			<label for="import-pack-name" class="font-semibold text-contrast">名称</label>
			<Input id="import-pack-name" v-model="name" />
		</div>
		<template #actions>
			<div class="flex items-center justify-end gap-2">
				<Button type="outlined" @click="modal?.hide()">取消</Button>
				<Button type="colored" color="brand" :disabled="!name.trim()" @click="proceed">导入</Button>
			</div>
		</template>
	</NewModal>
</template>
