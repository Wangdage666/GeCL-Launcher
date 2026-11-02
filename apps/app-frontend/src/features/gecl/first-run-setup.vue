<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { nextTick, onMounted, ref } from 'vue'
import AccountsCard from '@/features/minecraft-accounts/accounts-card.vue'
import { users } from '@/helpers/auth'
import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'

const props = defineProps<{ required: boolean }>()
const visible = ref(props.required)
const dialog = ref<HTMLDialogElement>()
const hasAccount = ref(false)
const busy = ref(false)
const error = ref('')
const sidebarAccounts = injectMinecraftAccounts()
async function refresh() {
	try {
		hasAccount.value = (await users()).length > 0
	} catch (e) {
		error.value = (e as { message?: string })?.message ?? String(e)
	}
}
async function next() {
	busy.value = true
	error.value = ''
	try {
		await invoke('plugin:gecl|finish_setup')
		await sidebarAccounts.value?.refreshValues()
		done()
	} catch (e) {
		error.value = (e as { message?: string }).message ?? String(e)
	} finally {
		busy.value = false
	}
}
function done() {
	dialog.value?.close()
	visible.value = false
}
onMounted(async () => {
	if (visible.value) {
		await nextTick()
		dialog.value?.showModal()
		await refresh()
	}
})
</script>

<template>
	<dialog
		v-if="visible"
		ref="dialog"
		class="setup-backdrop"
		aria-labelledby="setup-account-title"
		@cancel.prevent
	>
		<section class="setup-panel">
			<h1 id="setup-account-title">添加游玩账户</h1>
			<Suspense>
				<AccountsCard @change="refresh" />
				<template #fallback><p>正在读取账户…</p></template>
			</Suspense>
			<p v-if="error" role="alert" class="text-red">{{ error }}</p>
			<footer>
				<button class="exit" @click="getCurrentWindow().close()">退出</button
				><button :disabled="!hasAccount || busy" @click="next">完成设置</button>
			</footer>
		</section>
	</dialog>
</template>

<style scoped>
.setup-backdrop {
	position: fixed;
	inset: 0;
	z-index: 100;
	display: grid;
	place-items: center;
	background: transparent;
	border: 0;
	padding: 0;
	width: 100vw;
	height: 100vh;
	max-width: none;
	max-height: none;
}
.setup-backdrop::backdrop {
	background: #11182766;
}
.setup-panel {
	width: min(460px, calc(100vw - 64px));
	max-height: 80vh;
	overflow: auto;
	background: var(--color-surface-1, #fff);
	color: var(--color-contrast, #202832);
	padding: 28px;
	border-radius: 16px;
}
h1 {
	font-size: 22px;
	margin: 0 0 20px;
}
.step {
	font-size: 13px;
	color: var(--color-secondary, #596370);
	margin: 0 0 8px;
}
footer {
	display: flex;
	justify-content: flex-end;
	gap: 10px;
	margin-top: 24px;
}
button {
	font: inherit;
	cursor: pointer;
	border: 1px solid #1479ca;
	border-radius: 8px;
	padding: 9px 16px;
	background: #1479ca;
	color: #fff;
}
.exit {
	background: transparent;
	color: inherit;
	border-color: #cfd6dd;
}
button:disabled {
	opacity: 0.5;
	cursor: default;
}
button:focus-visible {
	outline: 2px solid #1479ca;
	outline-offset: 3px;
}
</style>
