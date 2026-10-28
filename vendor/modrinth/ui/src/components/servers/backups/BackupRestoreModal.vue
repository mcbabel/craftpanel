<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.restoreBackup)"
		fade="danger"
		width="500px"
	>
		<div class="flex flex-col gap-6">
			<Admonition
				v-if="ctx.isServerRunning.value"
				type="critical"
				:header="formatMessage(messages.serverRunningHeader)"
			>
				{{ formatMessage(messages.serverRunningBody) }}
			</Admonition>
			<Admonition v-else type="critical" :header="formatMessage(messages.filesReplacedHeader)">
				{{ formatMessage(messages.filesReplacedBody) }}
			</Admonition>

			<div v-if="currentBackup" class="flex flex-col gap-2">
				<span class="font-semibold text-contrast">{{ formatMessage(messages.backupLabel) }}</span>
				<BackupItem :backup="currentBackup" />
			</div>
		</div>

		<template #actions>
			<div class="flex gap-2 justify-end">
				<Button type="outlined" @click="modal?.hide()">
					<XIcon />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button
					v-tooltip="restoreDisabledTooltip"
					type="colored"
					color="red"
					:disabled="restoreDisabled"
					@click="restoreBackup"
				>
					<SpinnerIcon v-if="isRestoring" class="animate-spin" />
					<RotateCounterClockwiseIcon v-else />
					{{ formatMessage(isRestoring ? messages.restoring : messages.restoreBackup) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import type { Archon } from '@modrinth/api-client'
import { RotateCounterClockwiseIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'

import { Button } from '#ui/components/base/buttons'

import { defineMessages, useVIntl } from '../../../composables/i18n'
import {
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '../../../providers'
import { commonMessages } from '../../../utils'
import Admonition from '../../base/Admonition.vue'
import NewModal from '../../modal/NewModal.vue'
import BackupItem from './BackupItem.vue'

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	restoreBackup: {
		id: 'craftpanel.vendor.backups.restore-backup',
		defaultMessage: 'Restore backup',
	},
	serverRunningHeader: {
		id: 'craftpanel.vendor.backups.server-running-header',
		defaultMessage: 'Server is running',
	},
	serverRunningBody: {
		id: 'craftpanel.vendor.backups.server-running-body',
		defaultMessage: 'Stop the server before restoring a backup.',
	},
	filesReplacedHeader: {
		id: 'craftpanel.vendor.backups.files-replaced-header',
		defaultMessage: 'Your server files will be replaced',
	},
	filesReplacedBody: {
		id: 'craftpanel.vendor.backups.files-replaced-body',
		defaultMessage:
			'Restoring your server will replace the current world and server files. Any changes made since that backup will be permanently lost.',
	},
	backupLabel: {
		id: 'craftpanel.vendor.backups.backup-label',
		defaultMessage: 'Backup',
	},
	restoring: {
		id: 'craftpanel.vendor.backups.restoring',
		defaultMessage: 'Restoring...',
	},
	failedToRestore: {
		id: 'craftpanel.vendor.backups.failed-to-restore',
		defaultMessage: 'Failed to restore backup',
	},
	safetyBackupName: {
		id: 'craftpanel.vendor.backups.safety-backup-name',
		defaultMessage: 'Before restoring "{name}"',
	},
})
const client = injectModrinthClient()
const queryClient = useQueryClient()
const ctx = injectModrinthServerContext()

const props = withDefaults(
	defineProps<{
		canRestore?: boolean
		permissionDeniedMessage?: string
	}>(),
	{
		canRestore: true,
		permissionDeniedMessage: undefined,
	},
)

const backupsQueryKey = ['backups', 'queue', ctx.serverId]

function safetyBackupName(backupName: string) {
	const base = formatMessage(messages.safetyBackupName, { name: backupName })
	return base.slice(0, 92)
}

const restoreMutation = useMutation({
	mutationFn: ({ backupId, name }: { backupId: string; name: string }) =>
		client.archon.backups_queue_v1.restore(ctx.serverId, ctx.worldId.value!, backupId, { name }),
	onSuccess: () => queryClient.invalidateQueries({ queryKey: backupsQueryKey }),
})

const modal = ref<InstanceType<typeof NewModal>>()
const currentBackup = ref<Archon.BackupsQueue.v1.BackupQueueBackup | null>(null)
const isRestoring = ref(false)
const restoreDisabled = computed(
	() => isRestoring.value || ctx.isServerRunning.value || !props.canRestore,
)
const restoreDisabledTooltip = computed(() =>
	props.canRestore
		? undefined
		: (props.permissionDeniedMessage ?? formatMessage(commonMessages.noPermissionAction)),
)

function show(backup: Archon.BackupsQueue.v1.BackupQueueBackup) {
	currentBackup.value = backup
	modal.value?.show()
}

const restoreBackup = () => {
	if (!props.canRestore || !currentBackup.value || isRestoring.value) {
		if (!currentBackup.value) {
			addNotification({
				type: 'error',
				title: formatMessage(messages.failedToRestore),
				text: 'Current backup is null',
			})
		}
		return
	}

	isRestoring.value = true
	restoreMutation.mutate(
		{
			backupId: currentBackup.value.id,
			name: safetyBackupName(currentBackup.value.name),
		},
		{
			onSuccess: () => {
				modal.value?.hide()
			},
			onError: (error) => {
				const message = error instanceof Error ? error.message : String(error)
				addNotification({
					type: 'error',
					title: formatMessage(messages.failedToRestore),
					text: message,
				})
			},
			onSettled: () => {
				isRestoring.value = false
			},
		},
	)
}

defineExpose({
	show,
})
</script>
