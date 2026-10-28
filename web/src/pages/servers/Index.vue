<template>
	<div class="flex flex-col gap-6">
		<div class="flex flex-wrap items-center gap-3">
			<h1 class="m-0 mr-auto text-2xl font-extrabold text-contrast">
				{{ formatMessage(commonMessages.serversLabel) }}
			</h1>

			<Input
				v-if="servers.length > 0"
				v-model="query"
				type="search"
				:icon="SearchIcon"
				:placeholder="formatMessage(messages.searchPlaceholder)"
				clearable
				wrapper-class="w-full sm:w-72"
			/>

			<Button v-if="servers.length > 0" @click="openNaming(null, [])">
				<FolderIcon aria-hidden="true" />
				{{ formatMessage(messages.newGroup) }}
			</Button>

			<ButtonLink
				v-if="capabilities?.can_create_servers"
				:to="{ name: 'server-new' }"
				type="colored"
				color="brand"
			>
				<PlusIcon aria-hidden="true" />
				{{ formatMessage(messages.newServer) }}
			</ButtonLink>
			<Button v-else-if="capabilities" type="colored" color="brand" disabled>
				<PlusIcon aria-hidden="true" />
				{{ formatMessage(messages.newServer) }}
			</Button>
		</div>

		<Admonition v-if="blocked" type="warning" :body="blocked" />

		<div v-if="servers.length > 0" class="flex flex-wrap items-center gap-2">
			<TeleportOverflowMenu
				class="!h-[34px] !text-sm !font-medium"
				:icon-only="false"
				:circular="false"
				:label="formatMessage(messages.sortBy, { mode: formatMessage(sortLabels[sortMode]) })"
				:options="sortOptions"
			>
				<ArrowDownAZIcon v-if="sortMode === 'name'" aria-hidden="true" />
				<ClockArrowDownIcon v-else-if="sortMode === 'recent'" aria-hidden="true" />
				<CalendarClockIcon v-else-if="sortMode === 'newest'" aria-hidden="true" />
				<PlayIcon v-else aria-hidden="true" />
				{{ formatMessage(sortLabels[sortMode]) }}
				<DropdownIcon aria-hidden="true" />
			</TeleportOverflowMenu>
			<div class="h-6 w-px shrink-0 bg-surface-5 max-sm:hidden" />
			<FilterPills v-model="stateFilter" :options="stateFilterOptions">
				<template #all>{{ formatMessage(messages.filterAll) }}</template>
			</FilterPills>
			<label v-if="isAdmin" class="ml-auto flex cursor-pointer items-center gap-3 text-primary">
				<Toggle v-model="everyone" @update:model-value="() => void load(false)" />
				<span class="font-semibold">{{ formatMessage(messages.showEveryone) }}</span>
			</label>
		</div>

		<InvitationInbox @accepted="() => void load(false)" />

		<TransferInbox @accepted="() => void load(false)" />

		<Admonition v-if="failure" type="critical" :header="formatMessage(messages.loadFailed)">
			{{ failure }}
			<template #actions>
				<Button @click="() => load(true)">
					<RotateCounterClockwiseIcon aria-hidden="true" />
					{{ formatMessage(messages.retry) }}
				</Button>
			</template>
		</Admonition>

		<LoadingIndicator v-else-if="loading" />

		<EmptyState
			v-else-if="servers.length === 0"
			type="no-tasks"
			:heading="formatMessage(messages.emptyHeading)"
			:description="formatMessage(messages.emptyDescription)"
		>
			<template v-if="capabilities?.can_create_servers" #actions>
				<ButtonLink :to="{ name: 'server-new' }" type="colored" color="brand">
					<ServerPlusIcon aria-hidden="true" />
					{{ formatMessage(messages.newServer) }}
				</ButtonLink>
			</template>
		</EmptyState>

		<EmptyState
			v-else-if="(terms.length > 0 || stateFilter.length > 0) && rows.length === 0"
			type="no-search-result"
			:heading="formatMessage(messages.noMatchHeading)"
			:description="
				terms.length > 0
					? formatMessage(messages.noMatchDescription, { query })
					: formatMessage(messages.noStateMatch)
			"
		>
			<template #actions>
				<Button @click="clearFilters">
					<XIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.clearButton) }}
				</Button>
			</template>
		</EmptyState>

		<section v-for="section in sections" :key="section.key" class="flex flex-col gap-3">
			<div class="flex min-h-9 items-center gap-3">
				<Checkbox
					v-if="section.rows.length > 0"
					:model-value="allChosen(section)"
					:indeterminate="someChosen(section)"
					:description="formatMessage(messages.selectSection, { name: section.title })"
					@update:model-value="toggleSection(section)"
				/>
				<button
					type="button"
					class="flex min-w-0 cursor-pointer items-center gap-1.5 border-none bg-transparent p-0 text-secondary"
					:aria-expanded="!collapsed.has(section.key)"
					@click="toggleCollapsed(section.key)"
				>
					<ChevronRightIcon
						aria-hidden="true"
						class="size-5 shrink-0 transition-transform"
						:class="{ 'rotate-90': !collapsed.has(section.key) }"
					/>
					<h2 class="m-0 truncate text-lg font-semibold">{{ section.title }}</h2>
					<span class="text-sm font-medium">{{ section.rows.length }}</span>
				</button>
				<TeleportOverflowMenu
					v-if="section.group"
					class="ml-auto"
					type="quiet"
					:label="formatMessage(messages.groupMenu, { name: section.title })"
					:options="groupMenu(section.group)"
				>
					<MoreVerticalIcon aria-hidden="true" />
				</TeleportOverflowMenu>
			</div>

			<template v-if="!collapsed.has(section.key)">
				<p v-if="section.rows.length === 0" class="m-0 pl-8 text-secondary">
					{{ formatMessage(messages.emptyGroup) }}
				</p>

				<SmartClickable v-for="row in section.rows" :key="row.server.id">
					<template #clickable>
						<RouterLink
							:to="{ name: 'server-overview', params: { id: row.server.id } }"
							:aria-label="row.server.name"
							class="rounded-2xl"
						/>
					</template>

					<div
						class="smart-clickable:highlight-on-hover smart-clickable:outline-on-focus relative flex flex-row items-center gap-4 overflow-x-hidden rounded-2xl border-[1px] border-solid bg-bg-raised p-4 pl-5"
						:class="chosen.has(row.server.id) ? 'border-brand' : 'border-surface-4'"
					>
						<span
							aria-hidden="true"
							class="absolute inset-y-0 left-0 w-1.5"
							:class="stripes[row.power]"
						/>
						<Checkbox
							class="smart-clickable:allow-pointer-events"
							:model-value="chosen.has(row.server.id)"
							:description="formatMessage(messages.selectServer, { name: row.server.name })"
							@update:model-value="toggleChosen(row.server.id)"
						/>

						<PanelServerIcon
							:server-id="row.server.id"
							:disabled="row.server.status !== 'available' || row.power === 'stopped'"
						/>

						<div class="flex min-w-0 flex-col gap-1.5">
							<div class="flex flex-row flex-wrap items-center gap-2.5">
								<h3 class="m-0 truncate text-xl font-bold text-contrast">{{ row.server.name }}</h3>

								<div
									v-if="row.server.owner_id !== user?.id"
									class="flex min-w-0 items-center gap-1 rounded-full border border-solid border-surface-5 bg-surface-4 px-2 py-1 pr-2.5 text-sm font-medium text-primary"
								>
									<Avatar
										:src="row.owner?.avatar_url"
										:tint-by="row.owner?.username"
										size="1.25rem"
										circle
										no-shadow
									/>
									<span class="max-w-32 truncate">
										{{ row.owner?.username ?? formatMessage(commonMessages.unknownLabel) }}
									</span>
								</div>

								<Badge :type="row.state.label" :color="row.state.color" />
							</div>

							<ServerInfoLabels
								:server-data="{
									game: row.server.game,
									mc_version: row.server.game_version,
									loader: row.loaderName,
									loader_version: row.server.loader_version,
									net: row.server.net,
								}"
								:server-id="row.server.id"
								show-game-label
								:show-loader-label="!!row.loaderName"
								:linked="false"
								class="flex w-full flex-row flex-wrap items-center gap-2 text-primary"
							/>

							<div class="flex flex-row flex-wrap items-center gap-2 text-sm text-secondary">
								<CopyCode
									v-if="row.address"
									class="smart-clickable:allow-pointer-events"
									:text="row.address"
								/>
								<span v-else class="font-medium">
									{{ formatMessage(messages.portOnly, { port: row.server.net.port }) }}
								</span>
								<StatItem v-if="row.uptime">
									<ClockIcon aria-hidden="true" class="size-4" />
									{{ formatMessage(messages.upFor, { time: row.uptime }) }}
								</StatItem>
								<StatItem v-else-if="row.lastStarted && sortMode === 'recent'">
									<ClockArrowDownIcon aria-hidden="true" class="size-4" />
									{{ formatMessage(messages.lastStarted, { time: row.lastStarted }) }}
								</StatItem>
								<StatItem>
									<MemoryStickIcon aria-hidden="true" class="size-4" />
									{{ formatBytes(row.server.memory_mib * 1024 * 1024, 0) }}
								</StatItem>
								<StatItem>
									<DatabaseBackupIcon aria-hidden="true" class="size-4" />
									{{ row.server.used_backup_quota }} / {{ row.server.backup_quota }}
								</StatItem>
							</div>
						</div>

						<div v-if="row.operation" class="ml-auto hidden w-52 shrink-0 md:block">
							<ProgressBar
								full-width
								show-progress
								:progress="row.operation.progress"
								:waiting="row.operation.state === 'queued'"
								:label="formatMessage(operationLabels[row.operation.kind])"
								label-class="text-sm font-medium text-secondary"
							/>
						</div>
					</div>
				</SmartClickable>
			</template>
		</section>

		<FloatingActionBar
			:shown="chosenRows.length > 0 || bulk !== null"
			:aria-label="formatMessage(messages.selectionBar)"
		>
			<span class="px-3 py-2 text-base font-semibold text-contrast tabular-nums">
				{{
					bulk
						? formatMessage(messages.bulkProgress, { done: bulk.done, total: bulk.total })
						: formatMessage(messages.selectedCount, { count: chosenRows.length })
				}}
			</span>
			<div class="mx-0.5 h-6 w-px bg-surface-5" />
			<Button type="quiet" class="!text-primary" :disabled="bulk !== null" @click="clearChosen">
				<XIcon class="hidden cq-show-icon" aria-hidden="true" />
				<span class="bar-label">{{ formatMessage(messages.clearSelection) }}</span>
			</Button>

			<div class="ml-auto flex items-center gap-0.5">
				<Button
					type="quiet"
					:disabled="bulk !== null || eligible('start').length === 0"
					@click="runPower('start')"
				>
					<PlayIcon aria-hidden="true" />
					<span class="bar-label">{{ formatMessage(messages.start) }}</span>
				</Button>
				<Button
					type="quiet"
					:disabled="bulk !== null || eligible('restart').length === 0"
					@click="runPower('restart')"
				>
					<UpdatedIcon aria-hidden="true" />
					<span class="bar-label">{{ formatMessage(messages.restart) }}</span>
				</Button>
				<Button
					type="quiet"
					:disabled="bulk !== null || eligible('stop').length === 0"
					@click="runPower('stop')"
				>
					<StopCircleIcon aria-hidden="true" />
					<span class="bar-label">{{ formatMessage(messages.stop) }}</span>
				</Button>
				<TeleportOverflowMenu
					type="quiet"
					placement="top-end"
					:icon-only="false"
					:circular="false"
					:disabled="bulk !== null"
					:label="formatMessage(messages.moveToGroup)"
					:options="moveOptions"
				>
					<FolderIcon aria-hidden="true" />
					<span class="bar-label">{{ formatMessage(messages.group) }}</span>
				</TeleportOverflowMenu>
				<TeleportOverflowMenu
					type="quiet"
					placement="top-end"
					:disabled="bulk !== null"
					:label="formatMessage(messages.more)"
					:options="moreOptions"
				>
					<MoreVerticalIcon aria-hidden="true" />
				</TeleportOverflowMenu>
			</div>
		</FloatingActionBar>

		<NewModal
			ref="nameModal"
			:header="formatMessage(naming?.group ? messages.renameGroup : messages.newGroup)"
			width="28rem"
		>
			<form class="flex flex-col gap-4" @submit.prevent="saveName">
				<label class="flex flex-col gap-2">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.groupName) }}</span>
					<Input
						v-model="groupName"
						:maxlength="64"
						:placeholder="formatMessage(messages.groupNamePlaceholder)"
						autofocus
					/>
				</label>
				<p v-if="naming?.servers.length" class="m-0 text-secondary">
					{{ formatMessage(messages.newGroupWith, { count: naming.servers.length }) }}
				</p>
				<Admonition v-if="nameFailure" type="critical" :body="nameFailure" />
				<div class="mb-1 flex justify-end gap-2.5">
					<Button :disabled="savingName" @click="nameModal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						native-type="submit"
						:disabled="savingName || groupName.trim() === ''"
					>
						{{ formatMessage(commonMessages.saveButton) }}
					</Button>
				</div>
			</form>
		</NewModal>
	</div>
</template>

<script setup lang="ts">
import {
	ArrowDownAZIcon,
	CalendarClockIcon,
	ChevronRightIcon,
	ClockArrowDownIcon,
	ClockIcon,
	DropdownIcon,
	DatabaseBackupIcon,
	EditIcon,
	FolderIcon,
	MemoryStickIcon,
	MoreVerticalIcon,
	PlayIcon,
	PlusIcon,
	RotateCounterClockwiseIcon,
	SearchIcon,
	ServerPlusIcon,
	SkullIcon,
	StopCircleIcon,
	TrashIcon,
	UpdatedIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	Badge,
	Button,
	ButtonLink,
	type ButtonMenuOption,
	Checkbox,
	commonMessages,
	CopyCode,
	defineMessages,
	EmptyState,
	FilterPills,
	type FilterPillOption,
	FloatingActionBar,
	injectNotificationManager,
	Input,
	LoadingIndicator,
	type MessageDescriptor,
	NewModal,
	ProgressBar,
	ServerInfoLabels,
	SmartClickable,
	StatItem,
	TeleportOverflowMenu,
	Toggle,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { hasServerPermission } from '@modrinth/ui/src/composables/server-permissions.ts'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink } from 'vue-router'

import {
	api,
	type BusyReasonCode,
	isApiRequestError,
	type Operation,
	type OperationKind,
	type PowerAction,
	type PowerFacts,
	type PowerState,
	type Server,
	type ServerGroup,
	type Ulid,
	type UserRef,
} from '@/api'
import InvitationInbox from '@/components/InvitationInbox.vue'
import PanelServerIcon from '@/components/PanelServerIcon.vue'
import TransferInbox from '@/components/TransferInbox.vue'
import { useInvitations } from '@/composables/invitations'
import { useSession } from '@/composables/session'

const POLL_INTERVAL_MS = 5_000
const AT_ONCE = 4
const COLLAPSED_KEY = 'craftpanel.servers.collapsed'
const EVERYONE_KEY = 'craftpanel.servers.everyone'
const SORT_KEY = 'craftpanel.servers.sort'
const FILTER_KEY = 'craftpanel.servers.filter'

type SortMode = 'running' | 'recent' | 'name' | 'newest'

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const { user, isAdmin } = useSession()
const invitations = useInvitations()
const { addNotification } = injectNotificationManager()

const messages = defineMessages({
	newServer: {
		id: 'panel.servers.new-server',
		defaultMessage: 'New server',
	},
	searchPlaceholder: {
		id: 'panel.servers.search-placeholder',
		defaultMessage: 'Search servers...',
	},
	yourServers: {
		id: 'panel.servers.group.own',
		defaultMessage: 'Your servers',
	},
	sharedWithYou: {
		id: 'panel.servers.group.shared',
		defaultMessage: 'Shared with you',
	},
	serversOf: {
		id: 'panel.servers.group.of-user',
		defaultMessage: 'Servers of {name}',
	},
	showEveryone: {
		id: 'panel.servers.show-everyone',
		defaultMessage: 'Show the servers of every user',
	},
	emptyHeading: {
		id: 'panel.servers.empty.heading',
		defaultMessage: 'No servers yet',
	},
	emptyDescription: {
		id: 'panel.servers.empty.description',
		defaultMessage: 'Pick a loader and a version, and this machine will run it in a minute.',
	},
	noMatchHeading: {
		id: 'panel.servers.no-match.heading',
		defaultMessage: 'Nothing matches',
	},
	noMatchDescription: {
		id: 'panel.servers.no-match.description',
		defaultMessage: 'No server matches “{query}”.',
	},
	loadFailed: {
		id: 'panel.servers.load-failed',
		defaultMessage: 'The server list could not be loaded',
	},
	retry: {
		id: 'panel.servers.retry',
		defaultMessage: 'Try again',
	},
	portOnly: {
		id: 'panel.servers.port-only',
		defaultMessage: 'Port {port}',
	},
	unreachable: {
		id: 'panel.servers.error.unreachable',
		defaultMessage: 'The panel could not be reached.',
	},
	overLimit: {
		id: 'panel.servers.blocked.over-limit',
		defaultMessage:
			'Your servers already claim your whole memory budget. Free some up before making another one.',
	},
	systemUserNotReady: {
		id: 'panel.servers.blocked.system-user',
		defaultMessage:
			'Your system account is not ready yet. Nothing can run until an administrator has fixed it.',
	},
	installing: {
		id: 'panel.servers.state.installing',
		defaultMessage: 'Installing',
	},
	deleting: {
		id: 'panel.servers.state.deleting',
		defaultMessage: 'Deleting',
	},
	broken: {
		id: 'panel.servers.state.broken',
		defaultMessage: 'Broken',
	},
	setupPending: {
		id: 'panel.servers.state.setup-pending',
		defaultMessage: 'Needs setup',
	},
	syncingContent: {
		id: 'panel.servers.state.syncing-content',
		defaultMessage: 'Syncing content',
	},
	backingUp: {
		id: 'panel.servers.state.backup-creating',
		defaultMessage: 'Backing up',
	},
	restoring: {
		id: 'panel.servers.state.backup-restoring',
		defaultMessage: 'Restoring',
	},
	selectServer: {
		id: 'panel.servers.select-server',
		defaultMessage: 'Select {name}',
	},
	selectSection: {
		id: 'panel.servers.select-section',
		defaultMessage: 'Select every server in {name}',
	},
	clearSelection: {
		id: 'panel.servers.clear-selection',
		defaultMessage: 'Clear selection',
	},
	selectionBar: {
		id: 'panel.servers.selection-bar',
		defaultMessage: 'Actions for the selected servers',
	},
	selectedCount: {
		id: 'panel.servers.selected-count',
		defaultMessage: '{count, plural, one {# server} other {# servers}} selected',
	},
	bulkProgress: {
		id: 'panel.servers.bulk-progress',
		defaultMessage: 'Working on {done} of {total}...',
	},
	start: {
		id: 'panel.servers.bulk.start',
		defaultMessage: 'Start',
	},
	restart: {
		id: 'panel.servers.bulk.restart',
		defaultMessage: 'Restart',
	},
	stop: {
		id: 'panel.servers.bulk.stop',
		defaultMessage: 'Stop',
	},
	kill: {
		id: 'panel.servers.bulk.kill',
		defaultMessage: 'Kill',
	},
	more: {
		id: 'panel.servers.bulk.more',
		defaultMessage: 'More actions',
	},
	group: {
		id: 'panel.servers.bulk.group',
		defaultMessage: 'Group',
	},
	moveToGroup: {
		id: 'panel.servers.bulk.move-to-group',
		defaultMessage: 'Move to a group',
	},
	newGroupFromSelection: {
		id: 'panel.servers.bulk.new-group-from-selection',
		defaultMessage: 'New group from the selection',
	},
	removeFromGroup: {
		id: 'panel.servers.bulk.remove-from-group',
		defaultMessage: 'Take out of its group',
	},
	bulkDone: {
		id: 'panel.servers.bulk.done',
		defaultMessage: '{action} sent to {count, plural, one {# server} other {# servers}}',
	},
	bulkFailed: {
		id: 'panel.servers.bulk.failed',
		defaultMessage: '{count, plural, one {# server} other {# servers}} did not take it',
	},
	bulkSkipped: {
		id: 'panel.servers.bulk.skipped',
		defaultMessage:
			'{count, plural, one {# selected server was} other {# selected servers were}} left alone: already in that state, busy, or not yours to control.',
	},
	newGroup: {
		id: 'panel.servers.new-group',
		defaultMessage: 'New group',
	},
	renameGroup: {
		id: 'panel.servers.rename-group',
		defaultMessage: 'Rename group',
	},
	deleteGroup: {
		id: 'panel.servers.delete-group',
		defaultMessage: 'Delete group',
	},
	groupMenu: {
		id: 'panel.servers.group-menu',
		defaultMessage: 'Options for {name}',
	},
	groupName: {
		id: 'panel.servers.group-name',
		defaultMessage: 'Name',
	},
	groupNamePlaceholder: {
		id: 'panel.servers.group-name-placeholder',
		defaultMessage: 'Network, Tests, Friends...',
	},
	newGroupWith: {
		id: 'panel.servers.new-group-with',
		defaultMessage:
			'The {count, plural, one {selected server goes} other {# selected servers go}} straight into it.',
	},
	emptyGroup: {
		id: 'panel.servers.empty-group',
		defaultMessage: 'Nothing in here yet. Select servers and move them here with “Group”.',
	},
	groupFailed: {
		id: 'panel.servers.group-failed',
		defaultMessage: 'The groups could not be changed',
	},
	sortBy: {
		id: 'panel.servers.sort-by',
		defaultMessage: 'Sort by {mode}',
	},
	sortRunning: {
		id: 'panel.servers.sort.running',
		defaultMessage: 'Running first',
	},
	sortRecent: {
		id: 'panel.servers.sort.recent',
		defaultMessage: 'Last started',
	},
	sortName: {
		id: 'panel.servers.sort.name',
		defaultMessage: 'Name (A-Z)',
	},
	sortNewest: {
		id: 'panel.servers.sort.newest',
		defaultMessage: 'Newest first',
	},
	filterAll: {
		id: 'panel.servers.filter.all',
		defaultMessage: 'All',
	},
	filterRunning: {
		id: 'panel.servers.filter.running',
		defaultMessage: 'Running',
	},
	filterStopped: {
		id: 'panel.servers.filter.stopped',
		defaultMessage: 'Stopped',
	},
	filterCrashed: {
		id: 'panel.servers.filter.crashed',
		defaultMessage: 'Crashed',
	},
	noStateMatch: {
		id: 'panel.servers.no-state-match',
		defaultMessage: 'No server is in that state right now.',
	},
	upFor: {
		id: 'panel.servers.up-for',
		defaultMessage: 'Up {time}',
	},
	lastStarted: {
		id: 'panel.servers.last-started',
		defaultMessage: 'Started {time} ago',
	},
})

const sortLabels: Record<SortMode, MessageDescriptor> = {
	running: messages.sortRunning,
	recent: messages.sortRecent,
	name: messages.sortName,
	newest: messages.sortNewest,
}

const stripes: Record<PowerState, string> = {
	stopped: 'bg-transparent',
	starting: 'bg-orange',
	running: 'bg-brand',
	stopping: 'bg-orange',
	crashed: 'bg-red',
}

const operationLabels: Record<OperationKind, MessageDescriptor> = defineMessages({
	server_create: { id: 'panel.operation.server-create', defaultMessage: 'Setting up' },
	server_delete: { id: 'panel.operation.server-delete', defaultMessage: 'Deleting' },
	install_loader: { id: 'panel.operation.install-loader', defaultMessage: 'Installing loader' },
	repair_content: { id: 'panel.operation.repair-content', defaultMessage: 'Repairing' },
	reset_server: { id: 'panel.operation.reset-server', defaultMessage: 'Resetting' },
	install_modpack: { id: 'panel.operation.install-modpack', defaultMessage: 'Installing modpack' },
	install_content: { id: 'panel.operation.install-content', defaultMessage: 'Installing content' },
	update_content: { id: 'panel.operation.update-content', defaultMessage: 'Updating content' },
	change_game_version: {
		id: 'panel.operation.change-game-version',
		defaultMessage: 'Changing game version',
	},
	install_java: { id: 'panel.operation.install-java', defaultMessage: 'Installing Java' },
	backup_create: { id: 'panel.operation.backup-create', defaultMessage: 'Backing up' },
	backup_restore: { id: 'panel.operation.backup-restore', defaultMessage: 'Restoring backup' },
	unarchive: { id: 'panel.operation.unarchive', defaultMessage: 'Extracting' },
	archive: { id: 'panel.operation.archive', defaultMessage: 'Creating ZIP' },
})

const powerLabels: Record<PowerState, MessageDescriptor> = defineMessages({
	stopped: { id: 'panel.server.power.stopped', defaultMessage: 'Stopped' },
	starting: { id: 'panel.server.power.starting', defaultMessage: 'Starting' },
	running: { id: 'panel.server.power.running', defaultMessage: 'Running' },
	stopping: { id: 'panel.server.power.stopping', defaultMessage: 'Stopping' },
	crashed: { id: 'panel.server.power.crashed', defaultMessage: 'Crashed' },
})

const powerColors: Record<PowerState, string> = {
	stopped: 'gray',
	starting: 'orange',
	running: 'green',
	stopping: 'orange',
	crashed: 'red',
}

const actionLabels: Record<PowerAction, MessageDescriptor> = {
	start: messages.start,
	restart: messages.restart,
	stop: messages.stop,
	kill: messages.kill,
}

const busyLabels: Record<BusyReasonCode, MessageDescriptor> = {
	installing: messages.installing,
	syncing_content: messages.syncingContent,
	backup_creating: messages.backingUp,
	backup_restoring: messages.restoring,
	deleting: messages.deleting,
}

const servers = ref<Server[]>([])
const sharedIds = ref<Set<Ulid>>(new Set())
const owners = ref<Record<Ulid, UserRef>>({})
const power = ref<Record<Ulid, PowerFacts>>({})
const serverGroups = ref<ServerGroup[]>([])
const loaderNames = ref<Record<string, string>>({})
const operations = ref<Operation[]>([])
const busyReasons = ref<Record<Ulid, BusyReasonCode[]>>({})
const loading = ref(true)
const failure = ref<string | null>(null)
const query = ref('')
const chosen = ref<Set<Ulid>>(new Set())
const collapsed = ref<Set<string>>(new Set(remembered(COLLAPSED_KEY, [] as string[])))
const everyone = ref<boolean>(remembered(EVERYONE_KEY, false))
const bulk = ref<{ done: number; total: number } | null>(null)
const sortMode = ref<SortMode>(remembered<SortMode>(SORT_KEY, 'running'))
const stateFilter = ref<string[]>(remembered<string[]>(FILTER_KEY, []))
const now = ref(Date.now())

const nameModal = ref<InstanceType<typeof NewModal> | null>(null)
const naming = ref<{ group: ServerGroup | null; servers: Ulid[] } | null>(null)
const groupName = ref('')
const nameFailure = ref<string | null>(null)
const savingName = ref(false)

let timer: ReturnType<typeof setInterval> | null = null
let clock: ReturnType<typeof setInterval> | null = null

function remembered<T>(key: string, fallback: T): T {
	try {
		const stored = localStorage.getItem(key)
		return stored === null ? fallback : (JSON.parse(stored) as T)
	} catch {
		return fallback
	}
}

function remember(key: string, value: unknown): void {
	try {
		localStorage.setItem(key, JSON.stringify(value))
	} catch {
		return
	}
}

watch(everyone, (value) => remember(EVERYONE_KEY, value))
watch(sortMode, (value) => remember(SORT_KEY, value))
watch(stateFilter, (value) => remember(FILTER_KEY, value))

const sortOptions = computed<ButtonMenuOption[]>(() =>
	(
		[
			['running', PlayIcon],
			['recent', ClockArrowDownIcon],
			['name', ArrowDownAZIcon],
			['newest', CalendarClockIcon],
		] as const
	).map(([mode, icon]) => ({
		id: `sort-${mode}`,
		label: formatMessage(sortLabels[mode]),
		icon,
		action: () => (sortMode.value = mode),
	})),
)

const stateFilterOptions = computed<FilterPillOption[]>(() => [
	{ id: 'running', label: formatMessage(messages.filterRunning) },
	{ id: 'stopped', label: formatMessage(messages.filterStopped) },
	{ id: 'crashed', label: formatMessage(messages.filterCrashed) },
])

function powerOf(id: Ulid): PowerState {
	return power.value[id]?.state ?? 'stopped'
}

function filterKey(state: PowerState): string {
	if (state === 'starting' || state === 'stopping') return 'running'
	return state
}

function clearFilters(): void {
	query.value = ''
	stateFilter.value = []
}

function since(stamp: string | null | undefined): string | null {
	if (!stamp) return null
	const seconds = Math.max(0, Math.floor((now.value - Date.parse(stamp)) / 1000))
	if (Number.isNaN(seconds)) return null
	const days = Math.floor(seconds / 86_400)
	const hours = Math.floor((seconds % 86_400) / 3_600)
	const minutes = Math.floor((seconds % 3_600) / 60)
	if (days > 0) return `${days}d ${hours}h`
	if (hours > 0) return `${hours}h ${minutes}m`
	return `${minutes}m`
}

const RANK: Record<PowerState, number> = { running: 0, starting: 1, stopping: 2, crashed: 3, stopped: 4 }

function compare(left: Server, right: Server): number {
	const byName = left.name.localeCompare(right.name, undefined, { sensitivity: 'base' })
	switch (sortMode.value) {
		case 'running':
			return RANK[powerOf(left.id)] - RANK[powerOf(right.id)] || byName
		case 'recent': {
			const started = (id: Ulid) => power.value[id]?.last_started_at ?? ''
			return started(right.id).localeCompare(started(left.id)) || byName
		}
		case 'newest':
			return right.created_at.localeCompare(left.created_at) || byName
		default:
			return byName
	}
}

const capabilities = computed(() => user.value?.capabilities ?? null)
const showingEveryone = computed(() => isAdmin.value && everyone.value)

const blocked = computed(() => {
	switch (capabilities.value?.blocked_reason) {
		case 'over_limit':
			return formatMessage(messages.overLimit)
		case 'system_user_not_ready':
			return formatMessage(messages.systemUserNotReady)
		default:
			return null
	}
})

const terms = computed(() =>
	query.value
		.toLowerCase()
		.split(/\s+/)
		.filter((term) => term.length > 0),
)

function haystack(server: Server): string {
	const loader = server.loader ?? ''
	return [
		server.name,
		loader,
		loaderNames.value[loader] ?? '',
		server.game_version ?? '',
		server.game,
		owners.value[server.owner_id]?.username ?? '',
	]
		.join(' ')
		.toLowerCase()
}

const matching = computed(() =>
	servers.value
		.filter((server) => {
			if (stateFilter.value.length > 0 && !stateFilter.value.includes(filterKey(powerOf(server.id)))) {
				return false
			}
			if (terms.value.length === 0) return true
			const text = haystack(server)
			return terms.value.every((term) => text.includes(term))
		})
		.sort(compare),
)

interface Row {
	server: Server
	owner: UserRef | undefined
	operation: Operation | undefined
	state: { label: string; color: string }
	address: string
	loaderName: string | null
	power: PowerState
	uptime: string | null
	lastStarted: string | null
}

const rows = computed<Row[]>(() =>
	matching.value.map((server) => ({
		server,
		owner: owners.value[server.owner_id],
		operation: operations.value.find((operation) => operation.server_id === server.id),
		state: stateOf(server),
		address: server.net.ip ? `${server.net.ip}:${server.net.port}` : '',
		loaderName: server.loader ? (loaderNames.value[server.loader] ?? server.loader) : null,
		power: powerOf(server.id),
		uptime: powerOf(server.id) === 'running' ? since(power.value[server.id]?.running_since) : null,
		lastStarted: since(power.value[server.id]?.last_started_at),
	})),
)

interface Section {
	key: string
	title: string
	group: ServerGroup | null
	rows: Row[]
}

const placement = computed(() => {
	const known = new Set(servers.value.map((server) => server.id))
	const placed = new Map<Ulid, Ulid>()
	for (const group of serverGroups.value) {
		for (const id of group.server_ids) {
			if (known.has(id)) placed.set(id, group.id)
		}
	}
	return placed
})

const sections = computed<Section[]>(() => {
	const me = user.value?.id
	const placed = placement.value
	const loose = rows.value.filter((row) => !placed.has(row.server.id))
	const list: Section[] = serverGroups.value.map((group) => ({
		key: `group:${group.id}`,
		title: group.name,
		group,
		rows: rows.value.filter((row) => placed.get(row.server.id) === group.id),
	}))
	list.push({
		key: 'own',
		title: formatMessage(messages.yourServers),
		group: null,
		rows: loose.filter((row) => row.server.owner_id === me),
	})
	list.push({
		key: 'shared',
		title: formatMessage(messages.sharedWithYou),
		group: null,
		rows: loose.filter(
			(row) =>
				row.server.owner_id !== me && (!showingEveryone.value || sharedIds.value.has(row.server.id)),
		),
	})
	if (showingEveryone.value) {
		const others = new Map<Ulid, Row[]>()
		for (const row of loose) {
			if (row.server.owner_id === me || sharedIds.value.has(row.server.id)) continue
			others.set(row.server.owner_id, [...(others.get(row.server.owner_id) ?? []), row])
		}
		const byName = [...others.entries()].sort(([left], [right]) =>
			(owners.value[left]?.username ?? left).localeCompare(owners.value[right]?.username ?? right),
		)
		for (const [owner, ownRows] of byName) {
			list.push({
				key: `user:${owner}`,
				title: formatMessage(messages.serversOf, {
					name: owners.value[owner]?.username ?? formatMessage(commonMessages.unknownLabel),
				}),
				group: null,
				rows: ownRows,
			})
		}
	}
	return list.filter(
		(section) => section.rows.length > 0 || (section.group !== null && terms.value.length === 0),
	)
})

const chosenRows = computed(() => rows.value.filter((row) => chosen.value.has(row.server.id)))

function stateOf(server: Server): { label: string; color: string } {
	const busy = busyReasons.value[server.id]?.[0]
	if (server.status === 'deleting') return { label: formatMessage(messages.deleting), color: 'red' }
	if (server.status === 'installing' || busy === 'installing') {
		return { label: formatMessage(messages.installing), color: 'orange' }
	}
	if (busy) return { label: formatMessage(busyLabels[busy]), color: 'orange' }
	if (server.status === 'broken') return { label: formatMessage(messages.broken), color: 'red' }
	if (server.flows.intro) return { label: formatMessage(messages.setupPending), color: 'blue' }
	const state = powerOf(server.id)
	return { label: formatMessage(powerLabels[state]), color: powerColors[state] }
}

function toggleChosen(id: Ulid): void {
	const next = new Set(chosen.value)
	if (next.has(id)) next.delete(id)
	else next.add(id)
	chosen.value = next
}

function allChosen(section: Section): boolean {
	return section.rows.length > 0 && section.rows.every((row) => chosen.value.has(row.server.id))
}

function someChosen(section: Section): boolean {
	return !allChosen(section) && section.rows.some((row) => chosen.value.has(row.server.id))
}

function toggleSection(section: Section): void {
	const next = new Set(chosen.value)
	const all = allChosen(section)
	for (const row of section.rows) {
		if (all) next.delete(row.server.id)
		else next.add(row.server.id)
	}
	chosen.value = next
}

function clearChosen(): void {
	chosen.value = new Set()
}

function toggleCollapsed(key: string): void {
	const next = new Set(collapsed.value)
	if (next.has(key)) next.delete(key)
	else next.add(key)
	collapsed.value = next
	remember(COLLAPSED_KEY, [...next])
}

function eligible(action: PowerAction): Row[] {
	return chosenRows.value.filter((row) => {
		if (row.server.status !== 'available' || busyReasons.value[row.server.id]?.length) return false
		if (!hasServerPermission(row.server.current_user_permissions, 'POWER_ACTIONS')) return false
		const state = powerOf(row.server.id)
		if (action === 'start') return state === 'stopped' || state === 'crashed'
		if (action === 'kill') return state !== 'stopped' && state !== 'crashed'
		return state === 'running' || state === 'starting'
	})
}

async function runPower(action: PowerAction): Promise<void> {
	const targets = eligible(action)
	if (targets.length === 0 || bulk.value) return
	const skipped = chosenRows.value.length - targets.length
	const failures: string[] = []
	bulk.value = { done: 0, total: targets.length }
	const queue = [...targets]

	async function worker(): Promise<void> {
		for (let row = queue.shift(); row; row = queue.shift()) {
			try {
				const answer = await api.servers.power(row.server.id, { action })
				const before = power.value[row.server.id]
				power.value = {
					...power.value,
					[row.server.id]: {
						state: answer.power_state,
						running_since: before?.running_since ?? null,
						last_started_at: before?.last_started_at ?? null,
					},
				}
			} catch (error) {
				failures.push(`${row.server.name}: ${reason(error)}`)
			} finally {
				if (bulk.value) bulk.value = { ...bulk.value, done: bulk.value.done + 1 }
			}
		}
	}

	await Promise.all(Array.from({ length: Math.min(AT_ONCE, targets.length) }, () => worker()))
	bulk.value = null

	const label = formatMessage(actionLabels[action])
	if (failures.length > 0) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.bulkFailed, { count: failures.length }),
			text: failures.join('\n'),
		})
	}
	if (failures.length < targets.length) {
		addNotification({
			type: 'success',
			title: formatMessage(messages.bulkDone, { action: label, count: targets.length - failures.length }),
			text: skipped > 0 ? formatMessage(messages.bulkSkipped, { count: skipped }) : undefined,
		})
	}
	beat()
}

const moveOptions = computed<ButtonMenuOption[]>(() => {
	const ids = chosenRows.value.map((row) => row.server.id)
	return [
		...serverGroups.value.map((group) => ({
			id: `move-${group.id}`,
			label: group.name,
			icon: FolderIcon,
			action: () => void assign(group.id, ids),
		})),
		{ type: 'divider' as const, shown: serverGroups.value.length > 0 },
		{
			id: 'new-group',
			label: formatMessage(messages.newGroupFromSelection),
			icon: PlusIcon,
			action: () => openNaming(null, ids),
		},
		{
			id: 'ungroup',
			label: formatMessage(messages.removeFromGroup),
			icon: XIcon,
			shown: ids.some((id) => placement.value.has(id)),
			action: () => void assign(null, ids),
		},
	]
})

const moreOptions = computed<ButtonMenuOption[]>(() => [
	{
		id: 'kill',
		label: formatMessage(messages.kill),
		icon: SkullIcon,
		tone: 'red',
		disabled: eligible('kill').length === 0,
		action: () => void runPower('kill'),
	},
])

function groupMenu(group: ServerGroup): ButtonMenuOption[] {
	return [
		{
			id: 'rename',
			label: formatMessage(messages.renameGroup),
			icon: EditIcon,
			action: () => openNaming(group, []),
		},
		{
			id: 'delete',
			label: formatMessage(messages.deleteGroup),
			icon: TrashIcon,
			tone: 'red',
			action: () => void removeGroup(group),
		},
	]
}

function groupFailure(error: unknown): void {
	addNotification({ type: 'error', title: formatMessage(messages.groupFailed), text: reason(error) })
}

async function assign(group: Ulid | null, ids: Ulid[]): Promise<void> {
	try {
		serverGroups.value = (await api.serverGroups.assign({ server_ids: ids, group_id: group })).groups
	} catch (error) {
		groupFailure(error)
	}
}

async function removeGroup(group: ServerGroup): Promise<void> {
	try {
		serverGroups.value = (await api.serverGroups.remove(group.id)).groups
	} catch (error) {
		groupFailure(error)
	}
}

function openNaming(group: ServerGroup | null, ids: Ulid[]): void {
	naming.value = { group, servers: ids }
	groupName.value = group?.name ?? ''
	nameFailure.value = null
	nameModal.value?.show()
}

async function saveName(): Promise<void> {
	const current = naming.value
	const name = groupName.value.trim()
	if (!current || name === '' || savingName.value) return
	savingName.value = true
	nameFailure.value = null
	try {
		const answer = current.group
			? await api.serverGroups.update(current.group.id, { name })
			: await api.serverGroups.create({ name, server_ids: current.servers })
		serverGroups.value = answer.groups
		nameModal.value?.hide()
	} catch (error) {
		nameFailure.value = reason(error)
	} finally {
		savingName.value = false
	}
}

function reason(error: unknown): string {
	if (!isApiRequestError(error)) return formatMessage(messages.unreachable)
	return error.message || formatMessage(messages.unreachable)
}

async function load(initial: boolean): Promise<void> {
	if (initial) {
		loading.value = true
		failure.value = null
	}
	try {
		const wide = showingEveryone.value
		const [list, mine, active, groups] = await Promise.all([
			api.servers.list(wide ? { scope: 'all' } : {}),
			wide ? api.servers.list() : null,
			api.operations.listAll({ state: 'active' }),
			api.serverGroups.list().catch(() => null),
		])
		servers.value = list.servers
		owners.value = { ...list.users, ...(mine?.users ?? {}) }
		power.value = list.power ?? {}
		sharedIds.value = new Set(
			(mine ?? list).servers.filter((server) => server.owner_id !== user.value?.id).map((server) => server.id),
		)
		if (groups) serverGroups.value = groups.groups
		operations.value = active.operations
		busyReasons.value = active.busy_reasons_by_server
		const known = new Set(list.servers.map((server) => server.id))
		if ([...chosen.value].some((id) => !known.has(id))) {
			chosen.value = new Set([...chosen.value].filter((id) => known.has(id)))
		}
		failure.value = null
		beat()
	} catch (error) {
		if (initial) failure.value = reason(error)
	} finally {
		loading.value = false
	}
}

function settling(): boolean {
	return Object.values(power.value).some(
		(facts) => facts.state === 'starting' || facts.state === 'stopping',
	)
}

function beat(): void {
	if (operations.value.length === 0 && !settling()) {
		if (timer !== null) clearInterval(timer)
		timer = null
		return
	}
	timer ??= setInterval(() => void tick(), POLL_INTERVAL_MS)
}

async function tick(): Promise<void> {
	if (settling()) {
		await load(false)
		return
	}

	let active
	try {
		active = await api.operations.listAll({ state: 'active' })
	} catch {
		return
	}

	const still = new Set(active.operations.map((operation) => operation.id))
	const ended = operations.value.some((operation) => !still.has(operation.id))
	operations.value = active.operations
	busyReasons.value = active.busy_reasons_by_server

	if (ended) await load(false)
	else beat()
}

async function loadLoaderNames(): Promise<void> {
	const catalog = await api.settings.loaders().catch(() => null)
	if (!catalog) return
	loaderNames.value = Object.fromEntries(catalog.loaders.map((entry) => [entry.id, entry.name]))
}

onMounted(() => {
	void Promise.all([load(true), loadLoaderNames(), invitations.refresh()])
	clock = setInterval(() => (now.value = Date.now()), 30_000)
})

onBeforeUnmount(() => {
	if (timer !== null) clearInterval(timer)
	if (clock !== null) clearInterval(clock)
	timer = null
	clock = null
})
</script>
