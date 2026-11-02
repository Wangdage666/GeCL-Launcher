<template>
	<div
		v-if="accounts.length === 0"
		class="flex flex-col gap-3 bg-button-bg border border-solid border-surface-5 rounded-xl p-3 mt-2"
	>
		<span>{{ formatMessage(messages.notSignedIn) }}</span>
		<Button type="colored" color="brand" :disabled="loginDisabled" @click="login()">
			<LogInIcon v-if="!loginDisabled" />
			<SpinnerIcon v-else class="animate-spin" />
			{{ formatMessage(messages.signInToMinecraft) }}
		</Button>
		<Button @click="addingOffline = !addingOffline">离线账户</Button>
	</div>
	<Accordion
		v-else
		class="w-full mt-2 bg-button-bg border border-solid border-surface-5 rounded-xl overflow-clip"
		button-class="button-base w-full bg-transparent px-3 py-2 border-0 cursor-pointer"
		:open-by-default="false"
	>
		<template #title>
			<div class="flex gap-2 w-full min-w-0">
				<Avatar
					size="36px"
					:src="
						selectedAccount
							? avatarUrl
							: 'https://launcher-files.modrinth.com/assets/steve_head.png'
					"
				/>
				<div class="flex flex-col items-start w-full min-w-0">
					<span class="truncate w-full text-left">{{
						selectedAccount ? selectedAccount.profile.name : formatMessage(messages.selectAccount)
					}}</span>
					<span class="text-secondary text-xs">{{ selectedAccount?.account_type === 'offline' ? '离线账户' : '微软账户' }}</span>
				</div>
			</div>
		</template>
		<div class="bg-button-bg pt-1 pb-2 border-0 border-t border-solid border-surface-5">
			<template v-if="accounts.length > 0">
				<div v-for="account in accounts" :key="account.profile.id" class="flex gap-1 items-center">
					<button
						class="flex items-center flex-shrink flex-grow overflow-clip gap-2 p-2 border-0 bg-transparent cursor-pointer button-base min-w-0"
						@click="setAccount(account)"
					>
						<RadioButtonCheckedIcon
							v-if="selectedAccount && selectedAccount.profile.id === account.profile.id"
							class="w-5 h-5 text-brand shrink-0"
						/>
						<RadioButtonIcon v-else class="w-5 h-5 text-secondary shrink-0" />
						<Avatar :src="getAccountAvatarUrl(account)" size="24px" />
						<p
							class="m-0 truncate min-w-0"
							:class="
								selectedAccount && selectedAccount.profile.id === account.profile.id
									? 'text-contrast font-semibold'
									: 'text-primary'
							"
						>
							{{ account.profile.name }}{{ account.account_type === 'offline' ? '（离线）' : '' }}
						</p>
					</button>
					<IconButton
						v-tooltip="formatMessage(messages.removeAccount)"
						type="quiet"
						color="red"
						:label="formatMessage(messages.removeAccount)"
						class="mr-2 !bg-button-bg !text-primary ![box-shadow:var(--shadow-button)] hover:!bg-red focus-visible:!bg-red hover:!text-[var(--color-accent-contrast)] focus-visible:!text-[var(--color-accent-contrast)]"
						@click="logout(account.profile.id)"
					>
						<TrashIcon />
					</IconButton>
				</div>
			</template>
			<div class="flex flex-col gap-2 px-2 pt-2">
				<Button
					v-if="accounts.length > 0"
					class="w-full !bg-button-bg !text-primary ![box-shadow:var(--shadow-button)]"
					:disabled="loginDisabled"
					@click="login()"
				>
					<PlusIcon />
					{{ formatMessage(messages.addAccount) }}
				</Button>
				<Button @click="addingOffline = !addingOffline">添加离线账户</Button>
			</div>
		</div>
	</Accordion>
	<form v-if="addingOffline" class="flex flex-col gap-2 p-3" @submit.prevent="addOfflineAccount">
		<label for="gecl-offline-name">离线用户名</label>
		<input id="gecl-offline-name" v-model="offlineName" name="username" autocomplete="off" minlength="3" maxlength="16" pattern="[A-Za-z0-9_]{3,16}" required class="w-full box-border rounded border border-solid border-surface-5 bg-surface-2 p-2 text-contrast" />
		<p class="m-0 text-xs text-secondary">3–16 位英文字母、数字或下划线。离线账户用于单人游戏及允许离线连接的服务器，不代表正版身份。</p>
		<p v-if="offlineError" role="alert" class="m-0 text-sm text-red">{{ offlineError }}</p>
		<Button native-type="submit" :disabled="offlineBusy">{{ offlineBusy ? '正在保存…' : '添加并选择' }}</Button>
	</form>
	<section v-if="deviceLogin" class="flex flex-col gap-2 p-3" aria-label="微软账户登录">
		<p class="m-0">在微软登录页输入此代码：</p>
		<code class="select-text text-xl font-semibold tracking-widest">{{ deviceLogin.user_code }}</code>
		<Button @click="openMicrosoftLogin">打开微软登录页</Button>
		<Button @click="cancelLogin">取消登录</Button>
	</section>
</template>

<script setup lang="ts">
import {
	LogInIcon,
	PlusIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	SpinnerIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Accordion,
	Avatar,
	Button,
	defineMessages,
	IconButton,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import type { Ref } from 'vue'
import { computed, onUnmounted, ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'

import { useAppEvent } from '@/composables/use-app-event'
import type { MinecraftCredential } from '@/helpers/minecraft-credential'
import { toError } from '@/helpers/errors'
import { trackEvent } from '@/helpers/analytics'
import {
	get_default_user,
	add_offline_user,
	poll_device_login,
	cancel_device_login,
	login as login_flow,
	remove_user,
	set_default_user,
	users,
} from '@/helpers/auth'
import { getPlayerHeadUrl } from '@/helpers/rendering/player-head'
import type { Skin } from '@/helpers/skins'
import { get_available_skins } from '@/helpers/skins'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const emit = defineEmits<{
	change: []
}>()


const accounts: Ref<MinecraftCredential[]> = ref([])
const loginDisabled = ref(false)
const addingOffline = ref(false)
const offlineName = ref('')
const offlineError = ref('')
const offlineBusy = ref(false)
const deviceLogin = ref<{ id: string; user_code: string; verification_uri: string; interval: number } | null>(null)
let loginRequest = 0

async function addOfflineAccount() {
	if (offlineBusy.value) return
	offlineError.value = ''
	if (!/^[A-Za-z0-9_]{3,16}$/.test(offlineName.value)) {
		offlineError.value = '用户名须为 3–16 位英文字母、数字或下划线'
		return
	}
	cancelLogin()
	offlineBusy.value = true
	try {
		const user = await add_offline_user(offlineName.value)
		await setAccount(user)
		addingOffline.value = false
		offlineName.value = ''
	} catch (error) {
		offlineError.value = '保存离线账户失败，请检查数据目录权限后重试'
		handleError(toError(error))
	} finally {
		offlineBusy.value = false
	}
}

async function openMicrosoftLogin() {
	if (deviceLogin.value) await openUrl(deviceLogin.value.verification_uri).catch(handleError)
}

function cancelLogin() {
	loginRequest++
	const id = deviceLogin.value?.id
	deviceLogin.value = null
	loginDisabled.value = false
	if (id) void cancel_device_login(id).catch(handleError)
}

const defaultUser = ref<string | undefined>()
const equippedSkin = ref<Skin | null>(null)
const equippedHeadUrl = ref<string>()
let headRequest = 0

async function updateHeadUrl(skin: Skin | null) {
	const request = ++headRequest
	if (equippedHeadUrl.value) URL.revokeObjectURL(equippedHeadUrl.value)
	equippedHeadUrl.value = undefined
	if (!skin) return
	const url = await getPlayerHeadUrl(skin)
	if (request !== headRequest) URL.revokeObjectURL(url)
	else equippedHeadUrl.value = url
}

onUnmounted(() => {
	cancelLogin()
	headRequest++
	if (equippedHeadUrl.value) URL.revokeObjectURL(equippedHeadUrl.value)
})

async function refreshValues() {
	defaultUser.value = await get_default_user().catch(handleError)
	const userList = await users().catch(handleError)
	accounts.value = Array.isArray(userList) ? [...userList] : []
	accounts.value.sort((a, b) => (a.profile?.name ?? '').localeCompare(b.profile?.name ?? ''))

	if (accounts.value.find((account) => account.profile.id === defaultUser.value)?.account_type === 'offline') {
		equippedSkin.value = null
		await updateHeadUrl(null)
		return
	}
	try {
		const skins = await get_available_skins()
		equippedSkin.value = skins.find((skin) => skin.is_equipped) ?? null

		await updateHeadUrl(equippedSkin.value)
	} catch {
		equippedSkin.value = null
		void updateHeadUrl(null)
	}
}

async function setEquippedSkin(skin: Skin) {
	equippedSkin.value = skin

	try {
		await updateHeadUrl(skin)
	} catch (error) {
		console.warn('Failed to get head render for equipped skin:', error)
	}
}

function setLoginDisabled(value: boolean) {
	loginDisabled.value = value
}

defineExpose({
	cancelLogin,
	refreshValues,
	setEquippedSkin,
	setLoginDisabled,
	login,
	loginDisabled,
	selectedAccountId: defaultUser,
})

await refreshValues()

const selectedAccount = computed(() =>
	accounts.value.find((account) => account.profile.id === defaultUser.value),
)

const avatarUrl = computed(() => {
	if (selectedAccount.value?.account_type === 'offline') return '/gecl.svg'
	if (equippedSkin.value?.texture_key) {
		const cachedUrl = equippedHeadUrl.value
		if (cachedUrl) {
			return cachedUrl
		}
		return `https://mc-heads.net/avatar/${equippedSkin.value.texture_key}/128`
	}
	if (selectedAccount.value?.profile?.id) {
		return `https://mc-heads.net/avatar/${selectedAccount.value.profile.id}/128`
	}
	return 'https://launcher-files.modrinth.com/assets/steve_head.png'
})

function getAccountAvatarUrl(account: MinecraftCredential) {
	if (account.account_type === 'offline') return '/gecl.svg'
	if (
		account.profile.id === selectedAccount.value?.profile?.id &&
		equippedSkin.value?.texture_key
	) {
		const cachedUrl = equippedHeadUrl.value
		if (cachedUrl) {
			return cachedUrl
		}
	}
	return `https://mc-heads.net/avatar/${account.profile.id}/128`
}

async function setAccount(account: MinecraftCredential) {
	defaultUser.value = account.profile.id
	await set_default_user(account.profile.id).catch(handleError)
	await refreshValues()
	emit('change')
}

async function login() {
	if (loginDisabled.value) return
	const request = ++loginRequest
	loginDisabled.value = true
	let sessionId: string | undefined
	try {
		const flow = await login_flow()
		sessionId = flow.id
		if (request !== loginRequest) return
		deviceLogin.value = flow
		let interval = flow.interval
		const deadline = Date.now() + flow.expires_in * 1000
		while (request === loginRequest && Date.now() < deadline) {
			await new Promise((resolve) => setTimeout(resolve, interval * 1000))
			if (request !== loginRequest) return
			const result = await poll_device_login(flow.id)
			if (request !== loginRequest) return
			if (result.status === 'cancelled') return
			if (result.status === 'complete') {
				await setAccount(result.user)
				return
			}
			interval = result.interval
		}
		if (request === loginRequest) throw new Error('登录代码已过期，请重新登录')
	} catch (error) {
		if (request === loginRequest) handleError(toError(error))
	} finally {
		if (sessionId) void cancel_device_login(sessionId).catch(handleError)
		if (request === loginRequest) {
			deviceLogin.value = null
			loginDisabled.value = false
		}
	}
}

async function logout(id: string) {
	await remove_user(id).catch(handleError)
	await refreshValues()
	if (!selectedAccount.value && accounts.value.length > 0) {
		await setAccount(accounts.value[0])
	} else {
		emit('change')
	}
	trackEvent('AccountLogOut')
}

useAppEvent('process', async (e) => {
	if (e.event === 'launched') {
		await refreshValues()
	}
})

const messages = defineMessages({
	notSignedIn: {
		id: 'minecraft-account.not-signed-in',
		defaultMessage: 'Not signed in',
	},
	addAccount: {
		id: 'minecraft-account.add-account',
		defaultMessage: 'Add account',
	},
	removeAccount: {
		id: 'minecraft-account.remove-account',
		defaultMessage: 'Remove account',
	},
	selectAccount: {
		id: 'minecraft-account.select-account',
		defaultMessage: 'Select account',
	},
	minecraftAccount: {
		id: 'minecraft-account.label',
		defaultMessage: 'Minecraft account',
	},
	signInToMinecraft: {
		id: 'minecraft-account.sign-in',
		defaultMessage: 'Sign in to Minecraft',
	},
})
</script>
