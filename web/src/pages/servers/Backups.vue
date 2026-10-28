<template>
	<div class="flex flex-col gap-4">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="flex flex-col gap-1">
				<h1 class="m-0 text-2xl font-extrabold text-contrast">
					{{ formatMessage(messages.title) }}
				</h1>
				<span class="text-secondary">
					{{ formatMessage(messages.manualSlots, { used: manualUsed, total: manualSlots }) }}
				</span>
			</div>
			<Button
				v-tooltip="createTooltip"
				type="colored"
				color="brand"
				:disabled="createDisabled"
				@click="createModal?.show()"
			>
				<PlusIcon />
				{{ formatMessage(messages.create) }}
			</Button>
		</div>

		<Admonition
			v-if="query.isError.value"
			type="critical"
			:header="formatMessage(messages.loadFailed)"
			:body="errorText"
		/>

		<Admonition
			v-if="cannotBackUp"
			type="critical"
			:header="formatMessage(messages.driveBrokenHeader)"
			:body="formatMessage(driveBrokenBody)"
			show-actions-underneath
		>
			<template v-if="ownerCanConnect" #actions>
				<Button @click="router.push({ name: 'account' })">
					<SettingsIcon aria-hidden="true" />
					{{ formatMessage(messages.driveConnect) }}
				</Button>
			</template>
		</Admonition>

		<div class="flex flex-wrap items-center justify-between gap-3 rounded-xl bg-surface-2 p-3">
			<div class="flex min-w-0 flex-col">
				<span class="text-sm font-semibold text-contrast">
					{{ formatMessage(messages.autoTitle) }}
				</span>
				<span class="text-sm text-secondary">{{ autoSummary }}</span>
				<span v-if="autoEnabled && cannotBackUp" class="text-sm text-orange">
					{{ formatMessage(messages.scheduleImpossible) }}
				</span>
			</div>
			<Button @click="openBackupSettings">
				<SettingsIcon aria-hidden="true" />
				{{ formatMessage(messages.autoConfigure) }}
			</Button>
		</div>

		<div
			v-if="target && target.policy !== 'local_only'"
			class="flex flex-wrap items-center justify-between gap-3 rounded-xl bg-surface-2 p-3"
		>
			<div class="flex flex-col">
				<span class="text-sm font-semibold text-contrast">
					{{ formatMessage(messages.targetTitle) }}
				</span>
				<span class="text-sm text-secondary">
					{{
						formatMessage(
							target.effective_target === 'drive' ? messages.targetDrive : messages.targetLocal,
						)
					}}
					{{ target.reason === 'ok' ? '' : formatMessage(TARGET_REASONS[target.reason]) }}
				</span>
			</div>
			<Toggle
				v-if="targetIsChoosable(target)"
				:model-value="target.target === 'drive'"
				:disabled="!canManageBackups || savingTarget"
				@update:model-value="switchTarget"
			/>
		</div>

		<div class="flex flex-wrap items-center justify-between gap-3">
			<Chips v-model="filter" :items="filters" :format-label="filterLabel" />
			<div v-if="selected.size > 0" class="flex items-center gap-2">
				<span class="text-secondary">
					{{ formatMessage(messages.selected, { count: selected.size }) }}
				</span>
				<Button @click="selected.clear()">
					<XIcon />
					{{ formatMessage(messages.clearSelection) }}
				</Button>
				<Button
					v-tooltip="backupsTooltip"
					type="colored"
					color="red"
					:disabled="!canManageBackups"
					@click="deleteModal?.showBulk(selectedBackups)"
				>
					<TrashIcon />
					{{ formatMessage(commonMessages.deleteLabel) }}
				</Button>
			</div>
		</div>

		<LoadingIndicator v-if="query.isPending.value" />
		<EmptyState
			v-else-if="visibleBackups.length === 0 && !query.isError.value"
			:heading="formatMessage(messages.emptyHeading)"
			:description="formatMessage(messages.emptyDescription)"
		/>

		<div v-else class="flex flex-col gap-3">
			<div v-for="backup in visibleBackups" :key="backup.id" class="flex flex-col gap-1.5">
				<div class="flex flex-wrap items-center gap-2 sm:flex-nowrap sm:gap-3">
					<Checkbox
						:model-value="selected.has(backup.id)"
						:disabled="!canManageBackups"
						@update:model-value="toggle(backup.id)"
					/>
					<BackupItem class="min-w-0 flex-1" :backup="backup" />
					<div class="ml-auto flex shrink-0 items-center gap-1 max-sm:w-full max-sm:justify-end">
						<Button
							v-tooltip="restoreDisabled(backup)"
							:disabled="!!restoreDisabled(backup)"
							@click="restoreModal?.show(backup)"
						>
							<RefreshCwIcon aria-hidden="true" />
							{{ formatMessage(messages.restore) }}
						</Button>
						<TeleportOverflowMenu
							type="quiet"
							:label="formatMessage(commonMessages.moreOptionsButton)"
							:options="rowOptions(backup)"
						>
							<MoreHorizontalIcon aria-hidden="true" />
						</TeleportOverflowMenu>
						<IconButton
							v-if="openableInDrive(factsOf(backup))"
							type="quiet"
							:label="formatMessage(messages.openInDrive)"
							@click="openInDrive(backup)"
						>
							<ExternalIcon />
						</IconButton>
						<IconButton
							v-else
							type="quiet"
							:label="formatMessage(commonMessages.downloadButton)"
							@click="download(backup)"
						>
							<DownloadIcon />
						</IconButton>
					</div>
				</div>
				<div
					v-if="factsOf(backup).location === 'drive'"
					class="ml-9 flex flex-wrap items-center gap-2 text-sm"
				>
					<Badge
						v-if="noLongerOurs(factsOf(backup))"
						:type="formatMessage(messages.stateChanged)"
						color="red"
					/>
					<Badge
						v-else
						:type="formatMessage(FILE_STATES[factsOf(backup).state ?? 'present'])"
						:color="FILE_COLORS[factsOf(backup).state ?? 'present']"
					/>
					<span v-if="notRestorable(factsOf(backup))" class="text-secondary">
						{{ formatMessage(messages.driveGoneHint) }}
					</span>
					<span v-else-if="noLongerOurs(factsOf(backup))" class="text-red">
						{{ formatMessage(messages.driveChangedHint) }}
					</span>
					<span v-else-if="unconfirmed(factsOf(backup))" class="text-orange">
						{{ formatMessage(messages.driveUnconfirmedHint) }}
					</span>
				</div>

				<div v-if="runningOf(backup)" class="ml-9 flex flex-col gap-1">
					<span class="text-sm font-medium text-secondary">
						{{
							formatMessage(
								runningOf(backup) === 'restore' ? messages.restoring : messages.creating,
							)
						}}
					</span>
					<ProgressBar
						:progress="progressOf(backup)"
						:waiting="progressOf(backup) === 0"
						full-width
					/>
					<span v-if="holdupOf(backup)" class="text-sm text-secondary">
						{{ holdupOf(backup) }}
					</span>
					<span v-if="factsOf(backup).location === 'drive'" class="text-sm text-secondary">
						{{ carriesOn(backup) }}
					</span>
				</div>
				<Admonition
					v-else-if="failureOf(backup)"
					class="ml-9"
					type="critical"
					:header="
						abuseBlocked(backup)
							? formatMessage(messages.abuseHeader)
							: formatMessage(messages.failed)
					"
					:body="failureBody(backup)"
					show-actions-underneath
				>
					<template #actions>
						<Button
							v-if="abuseBlocked(backup)"
							v-tooltip="retryTooltip"
							:disabled="retryDisabled"
							@click="retry(backup.id, true)"
						>
							<DownloadIcon />
							{{ formatMessage(messages.abuseFetchAnyway) }}
						</Button>
						<Button
							v-else
							v-tooltip="retryTooltip"
							:disabled="retryDisabled"
							@click="retry(backup.id)"
						>
							<UpdatedIcon />
							{{ formatMessage(commonMessages.retryButton) }}
						</Button>
					</template>
				</Admonition>
			</div>
		</div>

		<Teleport to="body">
			<div class="relative z-[100]">
				<BackupCreateModal
					ref="createModal"
					:backups="backups"
					:can-create="!createDisabled"
					:permission-denied-message="permissionDeniedMessage"
				/>
				<BackupRenameModal
					ref="renameModal"
					:backups="backups"
					:can-rename="canManageBackups"
					:permission-denied-message="permissionDeniedMessage"
				/>
				<BackupRestoreModal
					ref="restoreModal"
					:can-restore="canManageBackups"
					:permission-denied-message="permissionDeniedMessage"
				/>
				<BackupDeleteModal
					ref="deleteModal"
					:can-delete="canManageBackups"
					:permission-denied-message="permissionDeniedMessage"
					@delete="removeOne"
					@bulk-delete="removeMany"
				/>
			</div>
		</Teleport>
	</div>
</template>

<script setup lang="ts">
import type { Archon } from '@modrinth/api-client'
import {
	ClipboardCopyIcon,
	DownloadIcon,
	EditIcon,
	ExternalIcon,
	MoreHorizontalIcon,
	PlusIcon,
	RefreshCwIcon,
	SettingsIcon,
	TrashIcon,
	UpdatedIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	BackupCreateModal,
	BackupDeleteModal,
	BackupItem,
	BackupRenameModal,
	BackupRestoreModal,
	Badge,
	Button,
	type ButtonMenuOption,
	Checkbox,
	Chips,
	commonMessages,
	defineMessages,
	EmptyState,
	IconButton,
	injectModrinthServerContext,
	injectNotificationManager,
	LoadingIndicator,
	ProgressBar,
	TeleportOverflowMenu,
	Toggle,
	useFormatDateTime,
	useServerBackupsQueue,
	useServerPermissions,
	useVIntl,
} from '@modrinth/ui'
import { useBackupIntervalFormat } from '@modrinth/ui/src/composables/hosting/backup-interval.ts'
import { computed, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import { api, type BackupSchedule, isApiRequestError, type OperationKind } from '@/api'
import { type BackupTarget, drive } from '@/api/drive'
import { useServerPage } from '@/composables/server-page'
import { useSession } from '@/composables/session'

import {
	backupImpossible,
	driveFactsOf,
	needsOwnersDrive,
	noLongerOurs,
	notRestorable,
	openableInDrive,
	targetIsChoosable,
	unconfirmed,
} from './backup-target'

type QueueBackup = Archon.BackupsQueue.v1.BackupQueueBackup

const BACKUP_KINDS = new Set<OperationKind>(['backup_create', 'backup_restore'])

const { formatMessage } = useVIntl()
const formatDateTime = useFormatDateTime({ dateStyle: 'medium', timeStyle: 'short' })
const { formatFrequency } = useBackupIntervalFormat()
const router = useRouter()
const { serverId, server, worldId } = injectModrinthServerContext()
const { addNotification } = injectNotificationManager()
const { canManageBackups, permissionDeniedMessage } = useServerPermissions()
const { user } = useSession()
const { operations } = useServerPage()

const messages = defineMessages({
	title: { id: 'craftpanel.backups.title', defaultMessage: 'Backups' },
	manualSlots: {
		id: 'craftpanel.backups.manual-slots',
		defaultMessage: '{used} of {total} manual backup slots used',
	},
	autoTitle: { id: 'craftpanel.backups.auto.title', defaultMessage: 'Auto backups' },
	autoOff: {
		id: 'craftpanel.backups.auto.off',
		defaultMessage: 'Off. Turn them on in the backup settings.',
	},
	autoOn: {
		id: 'craftpanel.backups.auto.on',
		defaultMessage: '{frequency}, keeps the newest {slots}',
	},
	autoNext: {
		id: 'craftpanel.backups.auto.next',
		defaultMessage: '{frequency}, keeps the newest {slots}. Next one {time}',
	},
	autoConfigure: { id: 'craftpanel.backups.auto.configure', defaultMessage: 'Configure' },
	create: { id: 'craftpanel.backups.create', defaultMessage: 'Create backup' },
	restore: { id: 'craftpanel.backups.restore', defaultMessage: 'Restore' },
	quotaFull: {
		id: 'craftpanel.backups.quota-full',
		defaultMessage: 'All manual backup slots are taken. Delete one or change the split in the backup settings.',
	},
	busy: { id: 'craftpanel.backups.busy', defaultMessage: 'A backup is already running.' },
	all: { id: 'craftpanel.backups.filter-all', defaultMessage: 'All' },
	manual: { id: 'craftpanel.backups.filter-manual', defaultMessage: 'Manual' },
	auto: { id: 'craftpanel.backups.filter-auto', defaultMessage: 'Auto' },
	selected: { id: 'craftpanel.backups.selected', defaultMessage: '{count} selected' },
	clearSelection: { id: 'craftpanel.backups.clear-selection', defaultMessage: 'Clear' },
	emptyHeading: { id: 'craftpanel.backups.empty-heading', defaultMessage: 'No backups yet' },
	emptyDescription: {
		id: 'craftpanel.backups.empty-description',
		defaultMessage: 'A backup holds the whole server directory and can be restored at any time.',
	},
	creating: { id: 'craftpanel.backups.creating', defaultMessage: 'Creating backup...' },
	restoring: { id: 'craftpanel.backups.restoring', defaultMessage: 'Restoring backup...' },
	failed: { id: 'craftpanel.backups.failed', defaultMessage: 'The last attempt failed' },
	abuseHeader: {
		id: 'craftpanel.backups.abuse-header',
		defaultMessage: 'Google has this archive down as harmful',
	},
	abuseWarning: {
		id: 'craftpanel.backups.abuse-warning',
		defaultMessage:
			'Google scanned this archive and calls it malware or spam. It hands it back only if you say outright that you accept the risk of downloading a possibly harmful file. If you did not pack a suspicious mod or plugin, restore an older backup instead.',
	},
	abuseFetchAnyway: {
		id: 'craftpanel.backups.abuse-fetch-anyway',
		defaultMessage: 'I accept the risk, fetch it anyway',
	},
	loadFailed: {
		id: 'craftpanel.backups.load-failed',
		defaultMessage: 'Failed to load the backups',
	},
	actionFailed: { id: 'craftpanel.backups.action-failed', defaultMessage: 'The action was refused' },
	downloadFailed: {
		id: 'craftpanel.backups.download-failed',
		defaultMessage: 'The backup cannot be downloaded',
	},
	archiveMissing: {
		id: 'craftpanel.backups.archive-missing',
		defaultMessage: 'Its archive is no longer on the server.',
	},
	deleted: { id: 'craftpanel.backups.deleted', defaultMessage: 'Backup deleted' },
	restoreRunning: {
		id: 'craftpanel.backups.restore-running',
		defaultMessage: 'Wait until the running backup operation is done.',
	},
	restoreNotDone: {
		id: 'craftpanel.backups.restore-not-done',
		defaultMessage: 'Only a finished backup can be restored.',
	},
	targetTitle: { id: 'craftpanel.backups.target', defaultMessage: 'Where new backups go' },
	targetLocal: {
		id: 'craftpanel.backups.target.local',
		defaultMessage: 'Onto this machine.',
	},
	targetDrive: {
		id: 'craftpanel.backups.target.drive',
		defaultMessage: 'Into the Google Drive of whoever owns this server.',
	},
	targetNotConfigured: {
		id: 'craftpanel.backups.target.not-configured',
		defaultMessage: 'The operator has not set up Google Drive.',
	},
	targetNotConnected: {
		id: 'craftpanel.backups.target.not-connected',
		defaultMessage: 'The owner of this server has not connected a Google account.',
	},
	targetPolicy: {
		id: 'craftpanel.backups.target.policy',
		defaultMessage: 'The operator decided this for the whole panel.',
	},
	driveBrokenHeader: {
		id: 'craftpanel.backups.drive-broken',
		defaultMessage: 'Backups of this server cannot be made right now',
	},
	driveBrokenNotConnected: {
		id: 'craftpanel.backups.drive-broken.not-connected',
		defaultMessage:
			'This server backs up into its owner’s Google Drive, and no Google account is connected. The owner can connect one on their account page; until then every backup of this server fails.',
	},
	driveBrokenNotConnectedOwner: {
		id: 'craftpanel.backups.drive-broken.not-connected-owner',
		defaultMessage:
			'You need to connect a Google account before this server can be backed up: its backups go into your Google Drive, and until one is connected every backup of this server fails.',
	},
	driveBrokenNotConfigured: {
		id: 'craftpanel.backups.drive-broken.not-configured',
		defaultMessage:
			'This server backs up into a Google Drive, but the operator has taken the panel’s Google credentials away. Nothing can be uploaded or fetched back until they are entered again.',
	},
	driveConnect: {
		id: 'craftpanel.backups.drive-connect',
		defaultMessage: 'Connect Google Drive',
	},
	retryImpossible: {
		id: 'craftpanel.backups.retry-impossible',
		defaultMessage: 'A new attempt cannot get anywhere until there is a Google Drive to put it in.',
	},
	scheduleImpossible: {
		id: 'craftpanel.backups.schedule-impossible',
		defaultMessage:
			'An automatic backup cannot be switched on while this server has no Google Drive to back up into — it would fail every time.',
	},
	openInDrive: { id: 'craftpanel.backups.open-in-drive', defaultMessage: 'Open in Google Drive' },
	stateInDrive: { id: 'craftpanel.backups.state.in-drive', defaultMessage: 'In Google Drive' },
	stateMissing: { id: 'craftpanel.backups.state.missing', defaultMessage: 'Gone from the Drive' },
	stateTrashed: { id: 'craftpanel.backups.state.trashed', defaultMessage: 'In the Drive’s bin' },
	stateUnreachable: {
		id: 'craftpanel.backups.state.unreachable',
		defaultMessage: 'Drive not connected',
	},
	driveGoneHint: {
		id: 'craftpanel.backups.drive-gone-hint',
		defaultMessage: 'Put it back in your Drive to restore it from here.',
	},
	driveUnconfirmedHint: {
		id: 'craftpanel.backups.drive-unconfirmed-hint',
		defaultMessage: 'Unconfirmed: Google named no checksum, so nothing says it arrived whole.',
	},
	stateChanged: {
		id: 'craftpanel.backups.state.changed',
		defaultMessage: 'Not this backup any more',
	},
	driveChangedHint: {
		id: 'craftpanel.backups.drive-changed-hint',
		defaultMessage:
			'The file in your Google Drive is no longer the archive the panel put there — something has written over it since. It cannot be restored from here.',
	},
	carryOnSending: {
		id: 'craftpanel.backups.carry-on-sending',
		defaultMessage:
			'A break does not have to start this over: as long as Google still holds the half-sent archive, the panel asks how far it got and sends only the rest.',
	},
	carryOnFetching: {
		id: 'craftpanel.backups.carry-on-fetching',
		defaultMessage:
			'A break does not have to start this over: as long as the archive in the Drive has not changed, what is already on the disk stays there and the panel fetches only the rest.',
	},
	targetSwitched: { id: 'craftpanel.backups.target-switched', defaultMessage: 'Target changed' },
})

const TARGET_REASONS = {
	ok: messages.targetLocal,
	not_configured: messages.targetNotConfigured,
	not_connected: messages.targetNotConnected,
	policy: messages.targetPolicy,
} as const

const FILE_STATES = {
	present: messages.stateInDrive,
	missing: messages.stateMissing,
	trashed: messages.stateTrashed,
	unreachable: messages.stateUnreachable,
} as const

const FILE_COLORS = {
	present: 'green',
	missing: 'red',
	trashed: 'orange',
	unreachable: 'orange',
} as const

const queue = useServerBackupsQueue(ref(serverId), worldId)
const { query, backups, activeOperations, activeOperationByBackupId, invalidate } = queue

type Filter = 'all' | 'manual' | 'auto'
const filters: Filter[] = ['all', 'manual', 'auto']
const filter = ref<Filter>('all')
const selected = reactive(new Set<string>())
const schedule = ref<BackupSchedule | null>(null)
const target = ref<BackupTarget | null>(null)
const savingTarget = ref(false)

const createModal = ref<InstanceType<typeof BackupCreateModal>>()
const renameModal = ref<InstanceType<typeof BackupRenameModal>>()
const restoreModal = ref<InstanceType<typeof BackupRestoreModal>>()
const deleteModal = ref<InstanceType<typeof BackupDeleteModal>>()

const manualSlots = computed(
	() => schedule.value?.manual_slots ?? server.value?.backup_quota ?? 0,
)
const manualUsed = computed(() => backups.value.filter((backup) => !backup.automated).length)
const autoEnabled = computed(() => (schedule.value?.automatic_slots ?? 0) > 0)
const autoSummary = computed(() => {
	const current = schedule.value
	if (current === null) return ''
	if (current.automatic_slots === 0) return formatMessage(messages.autoOff)
	const facts = {
		frequency: formatFrequency(current.interval_hours),
		slots: current.automatic_slots,
	}
	if (current.next_run_at === null) return formatMessage(messages.autoOn, facts)
	return formatMessage(messages.autoNext, { ...facts, time: formatDateTime(current.next_run_at) })
})
const errorText = computed(() =>
	query.error.value instanceof Error ? query.error.value.message : '',
)

const backupsTooltip = computed(() =>
	canManageBackups.value ? undefined : permissionDeniedMessage.value,
)

const cannotBackUp = computed(() => backupImpossible(target.value))

const isOwner = computed(() => user.value !== null && server.value?.owner_id === user.value.id)

const ownerCanConnect = computed(() => isOwner.value && needsOwnersDrive(target.value))

const driveBrokenBody = computed(() => {
	if (ownerCanConnect.value) return messages.driveBrokenNotConnectedOwner
	return target.value?.reason === 'not_configured'
		? messages.driveBrokenNotConfigured
		: messages.driveBrokenNotConnected
})

const createDisabled = computed(
	() =>
		!canManageBackups.value ||
		cannotBackUp.value ||
		manualUsed.value >= manualSlots.value ||
		queue.hasActiveCreate.value,
)
const createTooltip = computed(() => {
	if (!canManageBackups.value) return permissionDeniedMessage.value
	if (cannotBackUp.value) return formatMessage(driveBrokenBody.value)
	if (manualUsed.value >= manualSlots.value) return formatMessage(messages.quotaFull)
	if (queue.hasActiveCreate.value) return formatMessage(messages.busy)
	return undefined
})

const retryDisabled = computed(() => !canManageBackups.value || cannotBackUp.value)
const retryTooltip = computed(() => {
	if (!canManageBackups.value) return permissionDeniedMessage.value
	return cannotBackUp.value ? formatMessage(driveBrokenBody.value) : undefined
})

const visibleBackups = computed(() =>
	backups.value.filter((backup) => {
		if (filter.value === 'manual') return !backup.automated
		if (filter.value === 'auto') return backup.automated
		return true
	}),
)

const selectedBackups = computed(() => backups.value.filter((backup) => selected.has(backup.id)))

function filterLabel(value: Filter): string {
	return formatMessage(messages[value])
}

function rowOptions(backup: QueueBackup): ButtonMenuOption[] {
	const writeTooltip = canManageBackups.value ? undefined : permissionDeniedMessage.value
	return [
		{
			id: 'copy-id',
			label: formatMessage(commonMessages.copyIdButton),
			icon: ClipboardCopyIcon,
			action: () => void navigator.clipboard.writeText(backup.id),
		},
		{ type: 'divider' },
		{
			id: 'rename',
			label: formatMessage(commonMessages.renameButton),
			icon: EditIcon,
			disabled: !canManageBackups.value,
			tooltip: writeTooltip,
			action: () => renameModal.value?.show(backup),
		},
		{ type: 'divider' },
		{
			id: 'delete',
			label: formatMessage(commonMessages.deleteLabel),
			icon: TrashIcon,
			tone: 'red',
			disabled: !canManageBackups.value,
			tooltip: writeTooltip,
			action: () => deleteModal.value?.show(backup),
		},
	]
}

function toggle(id: string): void {
	if (selected.has(id)) selected.delete(id)
	else selected.add(id)
}

function runningOf(backup: QueueBackup): 'create' | 'restore' | null {
	return activeOperationByBackupId.value.get(backup.id)?.operation_type ?? null
}

const runningBackupOps = computed(() =>
	operations.value.filter(
		(entry) =>
			BACKUP_KINDS.has(entry.kind) && (entry.state === 'queued' || entry.state === 'ongoing'),
	),
)

function progressOf(backup: QueueBackup): number {
	return runningBackupOps.value.find((entry) => entry.target_id === backup.id)?.progress ?? 0
}

function holdupOf(backup: QueueBackup): string {
	return runningBackupOps.value.find((entry) => entry.target_id === backup.id)?.message ?? ''
}

function carriesOn(backup: QueueBackup): string {
	const fetching = runningOf(backup) === 'restore'
	return formatMessage(fetching ? messages.carryOnFetching : messages.carryOnSending)
}

function failureOf(backup: QueueBackup): string | null {
	const last = backup.history[0]
	if (!last || (last.state !== 'failed' && last.state !== 'timed_out')) return null
	return last.error ?? formatMessage(messages.failed)
}

function failureBody(backup: QueueBackup): string {
	const why = failureOf(backup) ?? ''
	if (abuseBlocked(backup)) return `${why} ${formatMessage(messages.abuseWarning)}`.trim()
	if (!cannotBackUp.value) return why
	return `${why} ${formatMessage(messages.retryImpossible)}`.trim()
}

function abuseBlocked(backup: QueueBackup): boolean {
	const last = operations.value
		.filter((entry) => entry.kind === 'backup_restore' && entry.target_id === backup.id)
		.sort((left, right) => right.created_at.localeCompare(left.created_at))[0]
	return last?.state === 'failed' && last.error?.code === 'drive_abuse_blocked'
}

function restoreDisabled(backup: QueueBackup): string | undefined {
	if (!canManageBackups.value) return permissionDeniedMessage.value
	if (backup.status !== 'done') return formatMessage(messages.restoreNotDone)
	if (activeOperations.value.length > 0) return formatMessage(messages.restoreRunning)
	if (notRestorable(driveFactsOf(backup))) return formatMessage(messages.driveGoneHint)
	if (noLongerOurs(driveFactsOf(backup))) return formatMessage(messages.driveChangedHint)
	return undefined
}

function report(cause: unknown): void {
	addNotification({
		type: 'error',
		title: formatMessage(messages.actionFailed),
		text: cause instanceof Error ? cause.message : undefined,
	})
}

async function download(backup: QueueBackup): Promise<void> {
	try {
		await api.backups.checkDownload(serverId, backup.id)
	} catch (cause) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.downloadFailed),
			text:
				isApiRequestError(cause) && cause.status === 404
					? formatMessage(messages.archiveMissing)
					: cause instanceof Error
						? cause.message
						: undefined,
		})
		return
	}
	window.location.assign(api.backups.downloadUrl(serverId, backup.id))
}

async function removeOne(backup: QueueBackup | undefined): Promise<void> {
	if (!backup) return
	try {
		await api.backups.remove(serverId, backup.id)
		selected.delete(backup.id)
		await invalidate()
		addNotification({ type: 'success', title: formatMessage(messages.deleted) })
	} catch (cause) {
		report(cause)
	}
}

async function removeMany(chosen: QueueBackup[]): Promise<void> {
	try {
		await api.backups.bulkDelete(serverId, { backup_ids: chosen.map((backup) => backup.id) })
		selected.clear()
		await invalidate()
	} catch (cause) {
		report(cause)
	}
}

async function retry(backupId: string, acknowledgeAbuse = false): Promise<void> {
	try {
		await api.backups.retry(serverId, backupId, acknowledgeAbuse ? { acknowledge_abuse: true } : {})
		await invalidate()
	} catch (cause) {
		report(cause)
	}
}

async function loadSchedule(): Promise<void> {
	try {
		schedule.value = await api.backups.schedule(serverId)
	} catch (cause) {
		report(cause)
	}
}

async function loadTarget(): Promise<void> {
	try {
		target.value = await drive.target(serverId)
	} catch {
		target.value = null
	}
}

async function switchTarget(wanted: boolean | undefined): Promise<void> {
	if (savingTarget.value) return
	savingTarget.value = true
	try {
		target.value = await drive.setTarget(serverId, wanted ? 'drive' : 'local')
		addNotification({ type: 'success', title: formatMessage(messages.targetSwitched) })
	} catch (cause) {
		report(cause)
		await loadTarget()
	} finally {
		savingTarget.value = false
	}
}

function factsOf(backup: QueueBackup) {
	return driveFactsOf(backup)
}

function openInDrive(backup: QueueBackup): void {
	const link = driveFactsOf(backup).webLink
	if (link !== null) window.open(link, '_blank', 'noopener,noreferrer')
}

function openBackupSettings(): void {
	void router.push({ name: 'server-settings', params: { id: serverId, section: 'backups' } })
}

watch(
	() => runningBackupOps.value.map((entry) => entry.id),
	(now, before) => {
		if (before.some((id) => !now.includes(id))) void invalidate()
	},
)

watch(
	() => backups.value.map((backup) => backup.id).join(),
	() => void loadSchedule(),
	{ immediate: true },
)
void loadTarget()
</script>
