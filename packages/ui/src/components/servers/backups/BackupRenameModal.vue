<template>
	<NewModal ref="modal" header="重命名备份" width="500px" @show="focusInput">
		<div class="flex flex-col gap-2">
			<label for="backup-name-input">
				<span class="text-lg font-semibold text-contrast"> 名称 </span>
			</label>
			<Input
				id="backup-name-input"
				ref="input"
				v-model="backupName"
				:placeholder="`Backup #${backupNumber}`"
				:maxlength="48"
				wrapper-class="w-full"
			/>
			<div v-if="nameExists" class="flex items-center gap-1">
				<IssuesIcon class="hidden text-orange sm:block" />
				<span class="text-sm text-orange"> 已有同名备份：'<span class="font-semibold">{{ trimmedName }}</span
					>'
				</span>
			</div>
		</div>
		<template #actions>
			<div class="flex gap-2 justify-end">
				<Button type="outlined" @click="hide">
					<XIcon /> 取消 </Button>
				<Button
					v-tooltip="renameDisabledTooltip"
					type="colored"
					color="brand"
					:disabled="renameDisabled"
					@click="renameBackup"
				>
					<template v-if="renameMutation.isPending.value">
						<SpinnerIcon class="animate-spin" /> 正在重命名… </template>
					<template v-else>
						<SaveIcon /> 保存更改 </template>
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import type { Archon } from '@modrinth/api-client'
import { IssuesIcon, SaveIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { computed, nextTick, ref } from 'vue'

import { Button } from '#ui/components/base/buttons'

import { useVIntl } from '../../../composables/i18n'
import {
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '../../../providers'
import { commonMessages } from '../../../utils'
import Input from '../../base/inputs/Input.vue'
import NewModal from '../../modal/NewModal.vue'

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const client = injectModrinthClient()
const queryClient = useQueryClient()
const ctx = injectModrinthServerContext()

const props = withDefaults(
	defineProps<{
		backups?: Archon.BackupsQueue.v1.BackupQueueBackup[]
		canRename?: boolean
		permissionDeniedMessage?: string
	}>(),
	{
		backups: undefined,
		canRename: true,
		permissionDeniedMessage: undefined,
	},
)

const backupsQueryKey = ['backups', 'queue', ctx.serverId]

const renameMutation = useMutation({
	mutationFn: ({ backupId, name }: { backupId: string; name: string }) =>
		client.archon.backups_v1.rename(ctx.serverId, ctx.worldId.value!, backupId, { name }),
	onSuccess: () => queryClient.invalidateQueries({ queryKey: backupsQueryKey }),
})

const modal = ref<InstanceType<typeof NewModal>>()
const input = ref<HTMLInputElement>()
const backupName = ref('')
const originalName = ref('')

const currentBackup = ref<Archon.BackupsQueue.v1.BackupQueueBackup | null>(null)

const trimmedName = computed(() => backupName.value.trim())

const nameExists = computed(() => {
	if (
		!props.backups ||
		trimmedName.value === originalName.value ||
		renameMutation.isPending.value
	) {
		return false
	}

	return props.backups.some(
		(backup) => backup.name.trim().toLowerCase() === trimmedName.value.toLowerCase(),
	)
})
const renameDisabled = computed(
	() => renameMutation.isPending.value || nameExists.value || !props.canRename,
)
const renameDisabledTooltip = computed(() =>
	props.canRename
		? undefined
		: (props.permissionDeniedMessage ?? formatMessage(commonMessages.noPermissionAction)),
)

const backupNumber = computed(
	() => (props.backups?.findIndex((b) => b.id === currentBackup.value?.id) ?? 0) + 1,
)

const focusInput = () => {
	nextTick(() => {
		setTimeout(() => {
			input.value?.focus()
		}, 100)
	})
}

function show(backup: Archon.BackupsQueue.v1.BackupQueueBackup) {
	currentBackup.value = backup
	backupName.value = backup.name
	originalName.value = backup.name
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

const renameBackup = () => {
	if (!props.canRename) return
	if (!currentBackup.value) {
		addNotification({
			type: 'error',
			title: "重命名备份失败",
			text: "未选择备份",
		})
		return
	}

	if (trimmedName.value === originalName.value) {
		hide()
		return
	}

	let newName = trimmedName.value
	if (newName.length === 0) {
		newName = `Backup #${backupNumber.value}`
	}

	renameMutation.mutate(
		{ backupId: currentBackup.value.id, name: newName },
		{
			onSuccess: () => {
				hide()
			},
			onError: (error) => {
				const message = error instanceof Error ? error.message : String(error)
				addNotification({ type: 'error', title: "重命名备份失败", text: message })
				hide()
			},
		},
	)
}

defineExpose({
	show,
	hide,
})
</script>
