<template>
	<div class="flex flex-col gap-8">
		<section class="members flex flex-col gap-3">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div class="flex flex-col gap-1">
					<h1 class="m-0 text-2xl font-extrabold text-contrast">
						{{ formatMessage(messages.membersTitle) }}
					</h1>
					<span class="text-secondary">{{ formatMessage(messages.membersHelp) }}</span>
				</div>
				<Button
					v-tooltip="manageTooltip"
					type="colored"
					color="brand"
					:disabled="!canManageUsers"
					@click="grantModal?.show()"
				>
					<UserPlusIcon />
					{{ formatMessage(messages.addUser) }}
				</Button>
			</div>

			<LoadingIndicator v-if="membersLoading" />
			<ErrorInformationCard
				v-else-if="membersError"
				:title="formatMessage(messages.membersFailed)"
				:description="membersError"
				:icon="IssuesIcon"
				:action="{
					label: formatMessage(commonMessages.retryButton),
					onClick: () => void loadMembers(),
				}"
			/>
			<AccessTable
				v-else
				:members="members"
				:roles="roles"
				:can-manage-users="canManageUsers"
				:permission-denied-message="permissionDeniedMessage"
				:user-profile-link="noProfileLink"
				@update-role="updateRole"
				@resend-invite="resendInvite"
				@cancel-invite="askRemove"
				@remove-member="askRemove"
			/>
		</section>

		<section class="flex flex-col gap-3">
			<div class="flex flex-col gap-1">
				<h2 class="m-0 text-xl font-extrabold text-contrast">
					{{ formatMessage(messages.auditTitle) }}
				</h2>
				<span class="text-secondary">{{ formatMessage(messages.auditHelp) }}</span>
			</div>

			<ErrorInformationCard
				v-if="auditError && auditEntries.length === 0"
				:title="formatMessage(messages.auditFailed)"
				:description="auditError"
				:icon="IssuesIcon"
				:action="{
					label: formatMessage(commonMessages.retryButton),
					onClick: () => void loadAudit(true),
				}"
			/>
			<AuditLogTable
				v-else
				v-model:filters="auditFilters"
				v-model:sort-direction="auditSort"
				v-model:timeframe-mode="timeframeMode"
				v-model:timeframe-preset="timeframePreset"
				v-model:timeframe-last-amount="timeframeLastAmount"
				v-model:timeframe-last-unit="timeframeLastUnit"
				v-model:timeframe-custom-start-date="timeframeStart"
				v-model:timeframe-custom-end-date="timeframeEnd"
				:entries="auditEntries"
				:has-active-external-filters="hasActiveExternalFilters"
				:has-more="nextOffset !== null"
				:loading="auditLoading"
				:loading-more="auditLoadingMore"
				:show-world-column="false"
				@load-more="() => void loadAudit(false)"
			/>
		</section>

		<Teleport to="body">
			<div class="relative z-[100]">
				<GrantAccessModal
					ref="grantModal"
					:members="members"
					:friend-ids="knownUserIds"
					:search-users="searchUsers"
					:can-grant="canManageUsers"
					:permission-denied-message="permissionDeniedMessage"
					:texts="grantTexts"
					:permissions-link="null"
					@grant="grant"
				/>
				<RemoveAccessModal
					ref="removeModal"
					:username="removalTarget?.user.username ?? ''"
					:avatar-url="removalTarget?.user.avatarUrl"
					:role="removalTarget?.role"
					:joined-at="removalTarget?.joinedAt ?? null"
					:pending="removalTarget?.pending ?? false"
					:should-cancel="removalTarget?.pending ?? false"
					:can-remove="canManageUsers"
					:permission-denied-message="permissionDeniedMessage"
					@remove="removeMember"
				/>
			</div>
		</Teleport>
	</div>
</template>

<script setup lang="ts">
import { IssuesIcon, UserPlusIcon } from '@modrinth/assets'
import {
	AccessTable,
	type AuditEventLookups,
	AuditLogTable,
	Button,
	commonMessages,
	defineMessages,
	ErrorInformationCard,
	GrantAccessModal,
	type GrantServerAccessPayload,
	injectModrinthServerContext,
	injectNotificationManager,
	LoadingIndicator,
	parseAuditEvent,
	RemoveAccessModal,
	type ServerAccessInviteSuggestion,
	type ServerAccessMember,
	type ServerAccessRole,
	type ServerAccessRoleOption,
	type ServerAccessUserProfileLink,
	type ServerAuditLogEntry,
	type ServerAuditLogFilters,
	type SortDirection,
	type TimeFrameLastUnit,
	type TimeFrameMode,
	type TimeFramePreset,
	useServerPermissions,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

import {
	api,
	type AuditEntry,
	type AuditLogPage,
	type AuditLogQuery,
	type ServerMember,
} from '@/api'

import { withoutProfileLinks } from './profile-links'
import { roleMessages } from './roles'

const PAGE_SIZE = 200

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const { serverId } = injectModrinthServerContext()
const { canManageUsers, permissionDeniedMessage } = useServerPermissions()

const messages = defineMessages({
	membersTitle: { id: 'craftpanel.access.members-title', defaultMessage: 'Access' },
	grantTarget: { id: 'craftpanel.access.grant-target', defaultMessage: 'Username' },
	grantPlaceholder: {
		id: 'craftpanel.access.grant-placeholder',
		defaultMessage: 'Search for a panel user',
	},
	grantHelp: {
		id: 'craftpanel.access.grant-help',
		defaultMessage: 'Only people with an account on this panel can be invited.',
	},
	membersHelp: {
		id: 'craftpanel.access.members-help',
		defaultMessage: 'Who may see and change this server. An invite has to be accepted first.',
	},
	addUser: { id: 'craftpanel.access.add-user', defaultMessage: 'Add user' },
	auditTitle: { id: 'craftpanel.access.audit-title', defaultMessage: 'Audit log' },
	auditHelp: {
		id: 'craftpanel.access.audit-help',
		defaultMessage: 'Everything that happened on this server, newest first.',
	},
	membersFailed: {
		id: 'craftpanel.access.members-failed',
		defaultMessage: 'Failed to load the members',
	},
	auditFailed: { id: 'craftpanel.access.audit-failed', defaultMessage: 'Failed to load the audit log' },
	actionFailed: { id: 'craftpanel.access.action-failed', defaultMessage: 'The change was refused' },
	resendCooldown: {
		id: 'craftpanel.access.resend-cooldown',
		defaultMessage: 'The invite was sent recently, try again in {seconds} seconds.',
	},
})

const roles: ServerAccessRoleOption[] = [
	{ value: 'owner', label: formatMessage(roleMessages.ownerRole) },
	{
		value: 'editor',
		label: formatMessage(roleMessages.editorRole),
		description: formatMessage(roleMessages.editorDescription),
	},
	{
		value: 'viewer',
		label: formatMessage(roleMessages.viewerRole),
		description: formatMessage(roleMessages.viewerDescription),
	},
]

const members = ref<ServerAccessMember[]>([])
const membersLoading = ref(true)
const membersError = ref<string | null>(null)
const removalTarget = ref<ServerAccessMember | null>(null)
const knownUserIds = ref<string[]>([])

const grantTexts = computed(() => ({
	header: formatMessage(messages.addUser),
	targetLabel: formatMessage(messages.grantTarget),
	targetPlaceholder: formatMessage(messages.grantPlaceholder),
	targetHelp: formatMessage(messages.grantHelp),
	editorRole: formatMessage(roleMessages.editorRole),
	editorDescription: formatMessage(roleMessages.editorDescription),
	viewerRole: formatMessage(roleMessages.viewerRole),
	viewerDescription: formatMessage(roleMessages.viewerDescription),
}))

const grantModal = ref<InstanceType<typeof GrantAccessModal>>()
const removeModal = ref<InstanceType<typeof RemoveAccessModal>>()

// AutoLink renders an object without a path as plain text; undefined would fall back to a
// Modrinth profile.
function noProfileLink(): ServerAccessUserProfileLink {
	return {}
}

const manageTooltip = computed(() =>
	canManageUsers.value ? undefined : permissionDeniedMessage.value,
)

function toAccessMember(member: ServerMember): ServerAccessMember {
	return {
		id: member.id,
		user: {
			id: member.user.id,
			username: member.user.username,
			avatarUrl: member.user.avatar_url ?? undefined,
		},
		role: member.role,
		joinedAt: member.joined_at,
		inviteResendAvailableAt: member.invite_resend_available_at,
		pending: member.pending,
		isOwner: member.is_owner,
	}
}

function describe(cause: unknown): string {
	return cause instanceof Error ? cause.message : String(cause)
}

function report(cause: unknown): void {
	addNotification({
		type: 'error',
		title: formatMessage(messages.actionFailed),
		text: describe(cause),
	})
}

async function loadMembers(): Promise<void> {
	membersLoading.value = true
	membersError.value = null
	try {
		const list = await api.access.members(serverId)
		members.value = list.members.map(toAccessMember)
	} catch (cause) {
		membersError.value = describe(cause)
	} finally {
		membersLoading.value = false
	}
}

async function updateRole(member: ServerAccessMember, role: ServerAccessRole): Promise<void> {
	if (role === 'owner') return
	try {
		await api.access.updateMember(serverId, member.user.id, { role })
		await loadMembers()
	} catch (cause) {
		report(cause)
	}
}

async function resendInvite(member: ServerAccessMember): Promise<void> {
	try {
		const result = await api.access.reinvite(serverId, member.user.id)
		if (!result.sent && result.cooldown_seconds !== null) {
			addNotification({
				type: 'warning',
				title: formatMessage(messages.resendCooldown, { seconds: result.cooldown_seconds }),
			})
		}
		members.value = members.value.map((entry) =>
			entry.id === member.id ? toAccessMember(result.member) : entry,
		)
	} catch (cause) {
		report(cause)
	}
}

function askRemove(member: ServerAccessMember): void {
	removalTarget.value = member
	removeModal.value?.show()
}

async function removeMember(): Promise<void> {
	const target = removalTarget.value
	if (target === null) return
	try {
		await api.access.removeMember(serverId, target.user.id)
		await loadMembers()
	} catch (cause) {
		report(cause)
	}
}

async function searchUsers(query: string): Promise<ServerAccessInviteSuggestion[]> {
	const result = await api.auth.searchUsers({ query, limit: 10 })
	const suggestions = result.users.map((user) => ({
		id: user.id,
		username: user.username,
		avatarUrl: user.avatar_url ?? undefined,
	}))
	knownUserIds.value = [...new Set([...knownUserIds.value, ...suggestions.map((one) => one.id)])]
	return suggestions
}

async function grant(payload: GrantServerAccessPayload): Promise<void> {
	try {
		await api.access.addMember(serverId, { user_id: payload.user.id, role: payload.role })
		await loadMembers()
	} catch (cause) {
		report(cause)
	}
}

const auditEntries = ref<ServerAuditLogEntry[]>([])
const auditLoading = ref(true)
const auditLoadingMore = ref(false)
const auditError = ref<string | null>(null)
const nextOffset = ref<number | null>(null)
const auditFilters = ref<ServerAuditLogFilters>({ userId: null, worldId: null })
const auditSort = ref<SortDirection>('desc')
const timeframeMode = ref<TimeFrameMode>('preset')
const timeframePreset = ref<TimeFramePreset>('all_time')
const timeframeLastAmount = ref(30)
const timeframeLastUnit = ref<TimeFrameLastUnit>('days')
const timeframeStart = ref('')
const timeframeEnd = ref('')

const lookups: AuditEventLookups = {
	serverId,
	users: {},
	addons: {},
	versions: {},
	worldById: new Map(),
	backupById: new Map(),
}

const DAY_MS = 24 * 60 * 60 * 1000
const UNIT_MS: Record<TimeFrameLastUnit, number> = {
	hours: 60 * 60 * 1000,
	days: DAY_MS,
	weeks: 7 * DAY_MS,
	months: 30 * DAY_MS,
}
const PRESET_DAYS: Partial<Record<TimeFramePreset, number>> = {
	last_7_days: 7,
	last_14_days: 14,
	last_30_days: 30,
	last_90_days: 90,
	last_180_days: 180,
}

function startOfToday(): Date {
	const today = new Date()
	today.setHours(0, 0, 0, 0)
	return today
}

function timeframeBounds(): { min?: string; max?: string } {
	if (timeframeMode.value === 'last') {
		const span = timeframeLastAmount.value * UNIT_MS[timeframeLastUnit.value]
		return { min: new Date(Date.now() - span).toISOString() }
	}
	if (timeframeMode.value !== 'preset') {
		return {
			min: timeframeStart.value ? new Date(timeframeStart.value).toISOString() : undefined,
			max: timeframeEnd.value ? new Date(timeframeEnd.value).toISOString() : undefined,
		}
	}
	const preset = timeframePreset.value
	if (preset === 'all_time') return {}
	if (preset === 'today') return { min: startOfToday().toISOString() }
	if (preset === 'yesterday') {
		const start = startOfToday()
		return {
			min: new Date(start.getTime() - DAY_MS).toISOString(),
			max: start.toISOString(),
		}
	}
	if (preset === 'year_to_date') {
		return { min: new Date(new Date().getFullYear(), 0, 1).toISOString() }
	}
	const days = PRESET_DAYS[preset] ?? 0
	return { min: new Date(Date.now() - days * DAY_MS).toISOString() }
}

const hasActiveExternalFilters = computed(() => {
	const bounds = timeframeBounds()
	return bounds.min !== undefined || bounds.max !== undefined || auditFilters.value.userId !== null
})

function absorb(page: AuditLogPage): void {
	Object.assign(lookups.users, page.users)
	Object.assign(lookups.addons, page.addons)
	Object.assign(lookups.versions, page.versions)
}

function toAuditEntry(entry: AuditEntry): ServerAuditLogEntry {
	const user = lookups.users[entry.actor.user_id]
	return {
		id: entry.id,
		actor: {
			id: entry.actor.user_id,
			username: user?.username ?? entry.actor.user_id.slice(0, 8),
			avatarUrl: user?.avatar_url ?? undefined,
		},
		world: null,
		event: withoutProfileLinks(parseAuditEvent(entry, lookups)),
		timestamp: entry.timestamp,
	}
}

async function loadAudit(reset: boolean): Promise<void> {
	if (!reset && nextOffset.value === null) return
	if (reset) auditLoading.value = true
	else auditLoadingMore.value = true
	auditError.value = null

	const bounds = timeframeBounds()
	const query: AuditLogQuery = {
		limit: PAGE_SIZE,
		offset: reset ? 0 : (nextOffset.value ?? 0),
		order: 'desc',
		...(bounds.min ? { min_datetime: bounds.min } : {}),
		...(bounds.max ? { max_datetime: bounds.max } : {}),
		...(auditFilters.value.userId ? { actor: [auditFilters.value.userId] } : {}),
	}

	try {
		const page = await api.access.auditLog(serverId, query)
		absorb(page)
		const mapped = page.data.map(toAuditEntry)
		auditEntries.value = reset ? mapped : [...auditEntries.value, ...mapped]
		nextOffset.value = page.next_offset
	} catch (cause) {
		if (reset) auditError.value = describe(cause)
		else report(cause)
	} finally {
		auditLoading.value = false
		auditLoadingMore.value = false
	}
}

watch(
	[
		() => auditFilters.value.userId,
		timeframeMode,
		timeframePreset,
		timeframeLastAmount,
		timeframeLastUnit,
		timeframeStart,
		timeframeEnd,
	],
	() => void loadAudit(true),
)

onMounted(() => {
	void loadMembers()
	void loadAudit(true)
})
</script>

<style scoped>
.members :deep(span.hover\:underline:hover) {
	text-decoration: none;
}

@media (max-width: 639px) {
	:deep([class*='grid-cols-[minmax(0,1.35fr)_7.75rem']) {
		grid-template-columns: minmax(0, 1fr) auto 3rem;
	}

	:deep([class*='grid-cols-[minmax(0,1.35fr)_7.75rem'] > :nth-child(3)) {
		display: none;
	}
}
</style>
