<template>
	<div class="flex flex-col gap-2.5">
		<span class="text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</span>
		<span>{{ formatMessage(messages.help) }}</span>

		<LoadingIndicator v-if="loading" />
		<ErrorInformationCard
			v-else-if="error"
			:title="formatMessage(messages.loadFailed)"
			:description="error"
			:icon="IssuesIcon"
			:action="{ label: formatMessage(commonMessages.retryButton), onClick: () => void load() }"
		/>
		<Table v-else :columns="columns" :data="logs" row-key="id" table-min-width="34rem">
			<template #empty-state>
				<EmptyState
					type="empty"
					:heading="formatMessage(messages.emptyTitle)"
					:description="formatMessage(messages.emptyHint)"
				/>
			</template>
			<template #cell-source="{ index }">
				<div class="flex min-w-0 flex-col">
					<span class="truncate font-semibold text-contrast">{{ sourceLabel(logAt(index)) }}</span>
					<a
						:href="logAt(index).url"
						target="_blank"
						rel="noopener noreferrer"
						class="truncate text-sm text-link hover:underline"
					>
						{{ logAt(index).url }}
					</a>
				</div>
			</template>
			<template #cell-shared="{ index }">
				<div class="flex min-w-0 flex-col">
					<span class="truncate">{{ logAt(index).shared_by?.username ?? formatMessage(messages.formerUser) }}</span>
					<span class="text-sm text-secondary">{{ relativeTime(logAt(index).created_at) }}</span>
				</div>
			</template>
			<template #cell-actions="{ index }">
				<div class="flex items-center justify-end gap-2">
					<IconButton
						v-tooltip="formatMessage(messages.copyLink)"
						type="quiet"
						:label="formatMessage(messages.copyLink)"
						@click="copy(logAt(index).url)"
					>
						<CopyIcon />
					</IconButton>
					<IconButton
						v-tooltip="mayRemove(logAt(index)) ? formatMessage(messages.remove) : formatMessage(messages.notYours)"
						type="quiet"
						:label="formatMessage(messages.remove)"
						:disabled="!mayRemove(logAt(index)) || removing === logAt(index).id"
						class="!text-red [&>svg]:!text-red"
						@click="startRemove(logAt(index))"
					>
						<TrashIcon />
					</IconButton>
				</div>
			</template>
		</Table>

		<Teleport to="body">
			<div class="relative z-[100]">
				<ConfirmModal
					ref="removeModal"
					:title="formatMessage(messages.removeTitle)"
					:description="formatMessage(messages.removeBody)"
					:proceed-label="formatMessage(messages.remove)"
					@proceed="remove"
				/>
			</div>
		</Teleport>
	</div>
</template>

<script setup lang="ts">
import { CopyIcon, IssuesIcon, TrashIcon } from '@modrinth/assets'
import {
	commonMessages,
	ConfirmModal,
	defineMessages,
	EmptyState,
	ErrorInformationCard,
	IconButton,
	injectModrinthServerContext,
	injectNotificationManager,
	LoadingIndicator,
	Table,
	type TableColumn,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'

import { api, type SharedLogEntry } from '@/api'
import { actionsColumnWidth, ICON_BUTTON_REM } from '@/components/table-widths'
import { useSession } from '@/composables/session'

const { formatMessage } = useVIntl()
const relativeTime = useRelativeTime()
const { addNotification } = injectNotificationManager()
const { serverId, server } = injectModrinthServerContext()
const { user, isAdmin } = useSession()

const messages = defineMessages({
	title: { id: 'craftpanel.shared-logs.title', defaultMessage: 'Shared logs' },
	help: {
		id: 'craftpanel.shared-logs.help',
		defaultMessage:
			'Logs shared from this server to mclo.gs. Anyone with the link can read them until you delete them here; otherwise mclo.gs removes a log 90 days after it was last viewed.',
	},
	loadFailed: {
		id: 'craftpanel.shared-logs.load-failed',
		defaultMessage: 'The shared logs could not be loaded',
	},
	emptyTitle: { id: 'craftpanel.shared-logs.empty-title', defaultMessage: 'No shared logs' },
	emptyHint: {
		id: 'craftpanel.shared-logs.empty-hint',
		defaultMessage: 'Logs you share from the console or the files show up here.',
	},
	source: { id: 'craftpanel.shared-logs.source', defaultMessage: 'Log' },
	shared: { id: 'craftpanel.shared-logs.shared', defaultMessage: 'Shared' },
	actions: { id: 'craftpanel.shared-logs.actions', defaultMessage: 'Actions' },
	console: { id: 'craftpanel.shared-logs.console', defaultMessage: 'Console' },
	formerUser: { id: 'craftpanel.shared-logs.former-user', defaultMessage: 'Deleted user' },
	copyLink: { id: 'craftpanel.shared-logs.copy-link', defaultMessage: 'Copy link' },
	copied: { id: 'craftpanel.shared-logs.copied', defaultMessage: 'Link copied' },
	remove: { id: 'craftpanel.shared-logs.remove', defaultMessage: 'Delete log' },
	notYours: {
		id: 'craftpanel.shared-logs.not-yours',
		defaultMessage: 'Only the person who shared it or the server owner can delete it',
	},
	removeTitle: {
		id: 'craftpanel.shared-logs.remove-title',
		defaultMessage: 'Delete this log from mclo.gs?',
	},
	removeBody: {
		id: 'craftpanel.shared-logs.remove-body',
		defaultMessage: 'The link stops working for everyone who has it. This cannot be undone.',
	},
	removed: { id: 'craftpanel.shared-logs.removed', defaultMessage: 'Log deleted' },
	removeFailed: {
		id: 'craftpanel.shared-logs.remove-failed',
		defaultMessage: 'The log could not be deleted',
	},
})

const logs = ref<SharedLogEntry[]>([])
const loading = ref(true)
const error = ref<string | null>(null)
const removing = ref<string | null>(null)
const pending = ref<SharedLogEntry | null>(null)
const removeModal = ref<InstanceType<typeof ConfirmModal>>()

const columns = computed<TableColumn[]>(() => [
	{ key: 'source', label: formatMessage(messages.source), width: '50%' },
	{ key: 'shared', label: formatMessage(messages.shared) },
	{
		key: 'actions',
		label: formatMessage(messages.actions),
		width: actionsColumnWidth([ICON_BUTTON_REM, ICON_BUTTON_REM]),
		align: 'right',
	},
])

function logAt(index: number): SharedLogEntry {
	return logs.value[index]
}

function sourceLabel(log: SharedLogEntry): string {
	return log.source === 'console' ? formatMessage(messages.console) : log.source
}

function mayRemove(log: SharedLogEntry): boolean {
	const me = user.value?.id
	return isAdmin.value || server.value?.owner_id === me || log.shared_by?.id === me
}

async function load(): Promise<void> {
	loading.value = true
	error.value = null
	try {
		logs.value = (await api.console.sharedLogs(serverId)).logs
	} catch (cause) {
		error.value = cause instanceof Error ? cause.message : String(cause)
	} finally {
		loading.value = false
	}
}

async function copy(url: string): Promise<void> {
	await navigator.clipboard.writeText(url)
	addNotification({ type: 'success', title: formatMessage(messages.copied), text: url })
}

function startRemove(log: SharedLogEntry): void {
	pending.value = log
	removeModal.value?.show()
}

async function remove(): Promise<void> {
	const log = pending.value
	if (!log) return
	removing.value = log.id
	try {
		await api.console.removeSharedLog(serverId, log.id)
		logs.value = logs.value.filter((other) => other.id !== log.id)
		addNotification({ type: 'success', title: formatMessage(messages.removed) })
	} catch (cause) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.removeFailed),
			text: cause instanceof Error ? cause.message : undefined,
		})
	} finally {
		removing.value = null
		pending.value = null
	}
}

onMounted(load)
</script>
