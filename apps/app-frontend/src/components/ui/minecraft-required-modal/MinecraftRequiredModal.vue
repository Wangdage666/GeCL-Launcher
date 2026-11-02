<script setup lang="ts">
import { NewModal } from '@modrinth/ui'
import { ref } from 'vue'
import AccountsCard from '@/features/minecraft-accounts/accounts-card.vue'
import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'
const modal = ref<InstanceType<typeof NewModal>>()
const accountVisible = ref(false)
const accounts = injectMinecraftAccounts()
function show() {
	accountVisible.value = true
	modal.value?.show()
}
async function selected() {
	await accounts.value?.refreshValues()
	modal.value?.hide()
}
defineExpose({ show })
</script>
<template>
	<NewModal
		ref="modal"
		header="选择游玩账户"
		max-width="440px"
		:on-hide="() => (accountVisible = false)"
	>
		<p class="m-0 mb-3 text-secondary">启动游戏前请选择账户，可使用微软账户或离线账户。</p>
		<Suspense v-if="accountVisible"><AccountsCard @change="selected" /></Suspense>
	</NewModal>
</template>
