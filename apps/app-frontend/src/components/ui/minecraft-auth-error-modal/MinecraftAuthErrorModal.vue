<script setup lang="ts">
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	LogInIcon,
	MessagesSquareIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { Admonition, Button, ButtonLink, Collapsible, IconButton, NewModal } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { handleSevereError } from '@/composables/use-error.js'
import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'

import { type MinecraftAuthError, minecraftAuthErrors } from './minecraft-auth-errors'

const modal = ref<InstanceType<typeof NewModal>>()
const rawError = ref<string>('')
const matchedError = ref<MinecraftAuthError | null>(null)
const debugCollapsed = ref(true)
const copied = ref(false)
const loadingSignIn = ref(false)
const minecraftAccounts = injectMinecraftAccounts()

function show(errorVal: { message?: string; code?: string }) {
	rawError.value = errorVal?.message ?? String(errorVal)

	matchedError.value = (errorVal?.code && minecraftAuthErrors[errorVal.code]) || null

	debugCollapsed.value = true
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({
	show,
	hide,
})

async function signInAgain() {
	try {
		loadingSignIn.value = true
		modal.value?.hide()
		await minecraftAccounts.value?.login()
		loadingSignIn.value = false
		modal.value?.hide()
	} catch (err) {
		loadingSignIn.value = false
		handleSevereError(err)
	}
}

const debugInfo = computed(() => rawError.value || "未提供错误信息。")

async function copyToClipboard(text: string) {
	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}
</script>

<template>
	<NewModal ref="modal" header="登录失败" :max-width="'548px'">
		<div class="flex flex-col gap-6">
			<Admonition
				type="warning"
				body="	无法登录微软账户，可能受到帐户权限或地区限制。"
			>
			</Admonition>

			<!-- Matched error details -->
			<div class="bg-surface-2 rounded-2xl p-4 px-5 flex flex-col gap-3">
				<template v-if="matchedError">
					<div class="flex flex-col gap-1.5">
						<h3 class="text-base font-bold m-0">可能原因</h3>
						<p class="text-sm text-secondary m-0">
							{{ matchedError.whatHappened }}
						</p>
					</div>

					<div class="flex flex-col gap-1.5">
						<h3 class="text-base font-bold m-0">处理方法</h3>
						<ol class="list-none flex flex-col gap-2 m-0 pl-0">
							<li
								v-for="(step, index) in matchedError.stepsToFix"
								:key="index"
								class="flex items-baseline gap-2"
							>
								<span
									class="inline-flex items-center justify-center shrink-0 w-5 h-5 rounded-full bg-surface-4 border border-solid border-surface-5 text-xs font-medium"
								>
									{{ index + 1 }}
								</span>
								<!-- eslint-disable-next-line vue/no-v-html -->
								<span
									class="text-sm [&_a]:text-info [&_a]:font-medium [&_a]:underline"
									v-html="step"
								/>
							</li>
						</ol>
					</div>
				</template>
				<template v-else>
					<div class="flex flex-col gap-1.5">
						<h3 class="text-base font-bold m-0">未知错误</h3>
						<p class="text-sm text-secondary m-0"> 暂时无法判断此错误的具体原因。 </p>
						<p class="text-sm text-secondary m-0"> 请尝试前往 <a
								class="text-info font-medium underline hover:underline"
								href="https://www.minecraft.net/en-us/login"
								>Minecraft 登录页</a
							> 完成登录，并按页面提示处理账户问题。反馈时请附上诊断信息。 </p>
					</div>
				</template>
			</div>

			<!-- Action buttons -->
			<div class="flex items-center gap-1">
				<ButtonLink href="https://support.modrinth.com" class="flex-1" @click="modal?.hide()">
					<MessagesSquareIcon /> 查看支持页面 </ButtonLink>
				<Button
					type="colored"
					color="brand"
					:disabled="loadingSignIn"
					class="flex-1"
					@click="signInAgain"
				>
					<LogInIcon /> 重新登录 </Button>
			</div>

			<div class="flex flex-col gap-2">
				<div class="w-full h-[1px] bg-surface-5"></div>

				<!-- Debug info -->
				<div class="overflow-clip">
					<button
						class="flex items-center justify-between w-full bg-transparent border-0 py-4 cursor-pointer"
						@click="debugCollapsed = !debugCollapsed"
					>
						<span class="flex items-center gap-2 text-contrast font-extrabold m-0">
							<WrenchIcon class="h-4 w-4" /> 诊断信息 </span>
						<DropdownIcon
							class="h-5 w-5 text-secondary transition-transform"
							:class="{ 'rotate-180': !debugCollapsed }"
						/>
					</button>
					<Collapsible :collapsed="debugCollapsed">
						<div
							class="p-3 bg-surface-2 rounded-2xl text-xs grid grid-cols-[1fr_auto] max-w-full items-start"
						>
							<div
								class="m-0 p-0 rounded-none bg-transparent text-sm font-mono break-words overflow-auto"
							>
								{{ debugInfo }}
							</div>
							<IconButton
								v-tooltip="'复制诊断信息'"
								:label="'复制诊断信息'"
								:disabled="copied"
								@click="copyToClipboard(debugInfo)"
							>
								<template v-if="copied"> <CheckIcon class="text-green" /> </template>
								<template v-else> <CopyIcon /> </template>
							</IconButton>
						</div>
					</Collapsible>
				</div>
			</div>
		</div>
	</NewModal>
</template>
