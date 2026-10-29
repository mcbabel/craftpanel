<template>
	<div class="mx-auto flex w-full max-w-[42rem] flex-col gap-4">
		<div class="flex items-center gap-3">
			<ButtonLink :to="{ name: 'servers' }" type="quiet" size="sm">
				<LeftArrowIcon aria-hidden="true" />
				{{ formatMessage(commonMessages.serversLabel) }}
			</ButtonLink>
		</div>

		<h1 class="m-0 text-2xl font-extrabold text-contrast">{{ formatMessage(messages.title) }}</h1>

		<Admonition
			v-if="stage === 'form' && blocked"
			type="warning"
			:header="formatMessage(messages.blockedHeader)"
			:body="blocked"
		/>

		<form v-else-if="stage === 'form'" class="flex flex-col gap-4" @submit.prevent="openSetup">
			<Card class="!mb-0 flex flex-col gap-1.5">
				<label class="text-sm font-semibold text-contrast" for="server-name">
					{{ formatMessage(messages.nameLabel) }}
				</label>
				<Input
					id="server-name"
					v-model="name"
					:maxlength="64"
					:placeholder="formatMessage(messages.namePlaceholder)"
					:disabled="busy"
					wrapper-class="w-full"
				/>
			</Card>

			<Card class="!mb-0 flex flex-col gap-3">
				<div class="flex flex-wrap items-baseline gap-2">
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.memoryLabel) }}
					</span>
					<span v-if="!unlimited" class="ml-auto text-sm text-secondary">
						{{
							formatMessage(messages.memoryBudget, {
								free: formatBytes(budgetLeft * 1024 * 1024, 0),
							})
						}}
					</span>
				</div>

				<LoadingIndicator v-if="ceiling === null" />
				<Admonition
					v-else-if="ceiling < MEMORY_MIN"
					type="warning"
					:body="formatMessage(messages.noMemoryLeft)"
				/>
				<Slider
					v-else
					v-model="memory"
					:min="MEMORY_MIN"
					:max="ceiling"
					:step="MEMORY_STEP"
					:disabled="busy"
					unit="MiB"
				/>

				<Admonition
					v-if="capacity.fallback"
					type="warning"
					:body="formatMessage(messages.hostUnavailable)"
				>
					<template #actions>
						<Button @click="() => loadHost()">
							<RotateCounterClockwiseIcon aria-hidden="true" />
							{{ formatMessage(messages.retry) }}
						</Button>
					</template>
				</Admonition>

				<Admonition
					v-if="!unlimited && memory > budgetLeft && (ceiling ?? 0) >= MEMORY_MIN"
					type="warning"
					:body="formatMessage(messages.overBudget)"
				/>
			</Card>

			<Card class="!mb-0 flex flex-col gap-4">
				<div class="flex flex-col gap-1.5">
					<Checkbox
						v-model="eula"
						:disabled="busy"
						:label="formatMessage(messages.eulaLabel)"
						label-class="text-sm text-contrast"
					/>
					<a
						href="https://aka.ms/MinecraftEULA"
						target="_blank"
						rel="noopener noreferrer"
						class="ml-8 w-fit text-sm text-brand"
					>
						{{ formatMessage(messages.eulaLink) }}
					</a>
				</div>

				<Admonition v-if="submitFailure" type="critical" :body="submitFailure" />

				<div class="flex flex-wrap justify-end gap-2">
					<ButtonLink :to="{ name: 'servers' }" :disabled="busy">
						{{ formatMessage(commonMessages.cancelButton) }}
					</ButtonLink>
					<Button
						native-type="submit"
						type="colored"
						color="brand"
						size="lg"
						:disabled="!submittable"
						:loading="busy"
					>
						<SpinnerIcon v-if="busy" class="animate-spin" />
						{{ formatMessage(messages.chooseInstallation) }}
						<RightArrowIcon v-if="!busy" aria-hidden="true" />
					</Button>
				</div>
			</Card>
		</form>

		<Card v-else class="!mb-0 flex flex-col gap-5">
			<div class="flex flex-col gap-1">
				<h2 class="m-0 text-lg font-semibold text-contrast">{{ created?.name }}</h2>
				<p class="m-0 text-secondary">{{ resultDescription }}</p>
			</div>

			<Admonition
				v-for="warning in warnings"
				:key="warning"
				type="warning"
				:body="formatMessage(warningMessages[warning])"
			/>

			<template v-if="stage === 'working'">
				<ProgressBar
					full-width
					show-progress
					:progress="uploading ? uploadProgress : (operation?.progress ?? 0)"
					:waiting="!uploading && operation?.state === 'queued'"
					:label="formatMessage(phaseLabel)"
					label-class="text-sm font-medium text-secondary"
				/>
				<p v-if="!uploading && operation?.message" class="m-0 truncate text-sm text-secondary">
					{{ operation.message }}
				</p>
			</template>

			<Admonition
				v-else-if="stage === 'failed'"
				type="critical"
				:header="formatMessage(messages.installFailed)"
				:body="operation?.error?.message"
			/>

			<Admonition v-if="submitFailure" type="critical" :body="submitFailure" />

			<div class="flex flex-wrap justify-end gap-2">
				<Button v-if="cancellable" :disabled="cancelling" @click="cancel">
					<XIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button v-if="stage === 'failed'" :disabled="retrying" @click="retry">
					<RotateCounterClockwiseIcon aria-hidden="true" />
					{{ formatMessage(messages.retry) }}
				</Button>
				<ButtonLink :to="{ name: 'servers' }">
					{{ formatMessage(commonMessages.serversLabel) }}
				</ButtonLink>
				<ButtonLink
					v-if="created"
					:to="{ name: 'server-overview', params: { id: created.id } }"
					type="colored"
					color="brand"
				>
					{{ formatMessage(messages.openServer) }}
					<RightArrowIcon aria-hidden="true" />
				</ButtonLink>
			</div>
		</Card>

		<ServerSetupFlow
			ref="setup"
			type="server-onboarding"
			:submit="create"
			@browse-modpacks="browseModpacks"
		/>
	</div>
</template>

<script setup lang="ts">
import {
	LeftArrowIcon,
	RightArrowIcon,
	RotateCounterClockwiseIcon,
	SpinnerIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Button,
	ButtonLink,
	Card,
	Checkbox,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	Input,
	LoadingIndicator,
	type MessageDescriptor,
	ProgressBar,
	Slider,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'
import { useRouter } from 'vue-router'

import {
	api,
	type CreateServerContent,
	isApiRequestError,
	type Operation,
	type OperationPhase,
	openServerSocket,
	type Server,
	type ServerSocket,
	type ServerWarning,
	type Ulid,
} from '@/api'
import ServerSetupFlow from '@/components/ServerSetupFlow.vue'
import { useSession } from '@/composables/session'
import { type SetupChoice, takeModpackPick } from '@/providers/creation-catalog'

import { type HostState, memoryCeiling } from './memory-ceiling'
import { keepDraft, takeDraft } from './new-server-draft'

const MEMORY_MIN = 512
const MEMORY_STEP = 256
const MEMORY_DEFAULT = 2048
const POLL_INTERVAL_MS = 5_000

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const router = useRouter()
const { addNotification } = injectNotificationManager()
const { user, isAdmin, refresh } = useSession()

const messages = defineMessages({
	title: {
		id: 'panel.server-new.title',
		defaultMessage: 'New server',
	},
	nameLabel: {
		id: 'panel.server-new.name.label',
		defaultMessage: 'Name',
	},
	namePlaceholder: {
		id: 'panel.server-new.name.placeholder',
		defaultMessage: 'My server',
	},
	memoryLabel: {
		id: 'panel.server-new.memory.label',
		defaultMessage: 'Memory',
	},
	memoryBudget: {
		id: 'panel.server-new.memory.budget',
		defaultMessage: '{free} left in your budget',
	},
	noMemoryLeft: {
		id: 'panel.server-new.memory.none-left',
		defaultMessage: 'Your budget has no room for another server. Free some memory first.',
	},
	hostUnavailable: {
		id: 'panel.server-new.memory.host-unavailable',
		defaultMessage:
			"The machine's memory could not be read, so the slider is not measured against it. Creating a server still works. Pick a value the machine can carry.",
	},
	overBudget: {
		id: 'panel.server-new.memory.over-budget',
		defaultMessage:
			'This goes past your own budget and into the machine. It will be allowed, but the memory is then overcommitted.',
	},
	eulaLabel: {
		id: 'panel.server-new.eula.label',
		defaultMessage: "I accept Minecraft's End User Licence Agreement.",
	},
	eulaLink: {
		id: 'panel.server-new.eula.link',
		defaultMessage: 'Read the agreement',
	},
	chooseInstallation: {
		id: 'panel.server-new.choose-installation',
		defaultMessage: 'Choose what to install',
	},
	createFailed: {
		id: 'panel.server-new.create-failed',
		defaultMessage: 'The server could not be created',
	},
	openServer: {
		id: 'panel.server-new.open-server',
		defaultMessage: 'Open server',
	},
	retry: {
		id: 'panel.server-new.retry',
		defaultMessage: 'Try again',
	},
	installFailed: {
		id: 'panel.server-new.install-failed',
		defaultMessage: 'Setting the server up failed',
	},
	working: {
		id: 'panel.server-new.working',
		defaultMessage: 'Your server is being set up. This usually takes a minute or two.',
	},
	uploading: {
		id: 'panel.server-new.uploading',
		defaultMessage: 'Uploading the modpack',
	},
	uploadFailed: {
		id: 'panel.server-new.upload-failed',
		defaultMessage: 'The modpack could not be uploaded: {reason}',
	},
	cancelled: {
		id: 'panel.server-new.cancelled',
		defaultMessage:
			'Setup was cancelled. The server exists but has nothing on it yet. Set it up again or delete it.',
	},
	failedDescription: {
		id: 'panel.server-new.failed-description',
		defaultMessage: 'The server exists but has nothing on it yet.',
	},
	blockedHeader: {
		id: 'panel.server-new.blocked.header',
		defaultMessage: 'You cannot create a server right now',
	},
	overLimit: {
		id: 'panel.server-new.blocked.over-limit',
		defaultMessage:
			'Your servers already claim your whole memory budget. Free some up before making another one.',
	},
	systemUserNotReady: {
		id: 'panel.server-new.blocked.system-user',
		defaultMessage:
			'Your system account is not ready yet. Nothing can run until an administrator has fixed it.',
	},
	unreachable: {
		id: 'panel.server-new.error.unreachable',
		defaultMessage: 'The panel could not be reached.',
	},
})

const submitFailures: Record<string, MessageDescriptor> = defineMessages({
	invalid_request: {
		id: 'panel.server-new.error.invalid-request',
		defaultMessage: 'The name may not be empty or longer than 64 characters.',
	},
	budget_exceeded: {
		id: 'panel.server-new.error.budget-exceeded',
		defaultMessage: 'That is more memory than your budget has left.',
	},
	over_limit: {
		id: 'panel.server-new.error.over-limit',
		defaultMessage: 'Your servers already claim more memory than your budget allows.',
	},
	no_free_port: {
		id: 'panel.server-new.error.no-free-port',
		defaultMessage: 'No free port is left from 25565 up.',
	},
	port_in_use: {
		id: 'panel.server-new.error.port-in-use',
		defaultMessage: 'The port is taken. Try again.',
	},
	unsupported_game_version: {
		id: 'panel.server-new.error.unsupported-game-version',
		defaultMessage: 'This loader does not offer that game version.',
	},
	unknown_loader: {
		id: 'panel.server-new.error.unknown-loader',
		defaultMessage: 'This loader is unknown to the panel.',
	},
	upstream_unavailable: {
		id: 'panel.server-new.error.upstream-unavailable',
		defaultMessage: 'The download source could not be reached. Try again in a moment.',
	},
	network_unreachable: {
		id: 'panel.server-new.error.network-unreachable',
		defaultMessage: 'The panel could not be reached.',
	},
	modpack_loader_unsupported: {
		id: 'panel.server-new.error.modpack-loader-unsupported',
		defaultMessage: 'This modpack needs a loader the panel cannot install yet.',
	},
	external_services_disabled: {
		id: 'panel.server-new.error.external-services-disabled',
		defaultMessage: 'An administrator has switched off downloads from Modrinth.',
	},
	invalid_modpack: {
		id: 'panel.server-new.error.invalid-modpack',
		defaultMessage: 'This file is not a modpack the panel can read.',
	},
	file_too_large: {
		id: 'panel.server-new.error.file-too-large',
		defaultMessage: 'The modpack is larger than the panel accepts.',
	},
})

const warningMessages: Record<ServerWarning, MessageDescriptor> = defineMessages({
	memory_overcommitted: {
		id: 'panel.server-new.warning.memory-overcommitted',
		defaultMessage: 'The machine now promises more memory than it has.',
	},
	properties_will_be_ignored: {
		id: 'panel.server-new.warning.properties-ignored',
		defaultMessage: 'This loader has no server.properties, so the world settings were dropped.',
	},
})

const phaseMessages: Record<OperationPhase, MessageDescriptor> = defineMessages({
	analyzing: { id: 'panel.phase.analyzing', defaultMessage: 'Resolving the version' },
	installing_loader: { id: 'panel.phase.installing-loader', defaultMessage: 'Downloading' },
	verifying: { id: 'panel.phase.verifying', defaultMessage: 'Checking the download' },
	running_installer: { id: 'panel.phase.running-installer', defaultMessage: 'Running the installer' },
	installing_java: { id: 'panel.phase.installing-java', defaultMessage: 'Fetching Java' },
	installing_pack: { id: 'panel.phase.installing-pack', defaultMessage: 'Unpacking the modpack' },
	addons: { id: 'panel.phase.addons', defaultMessage: 'Fetching content' },
	writing_config: { id: 'panel.phase.writing-config', defaultMessage: 'Writing the configuration' },
})

const setup = useTemplateRef<InstanceType<typeof ServerSetupFlow>>('setup')

const name = ref('')
const memory = ref(MEMORY_DEFAULT)
const eula = ref(false)

const stage = ref<'form' | 'working' | 'failed' | 'cancelled'>('form')
const busy = ref(false)
const submitFailure = ref<string | null>(null)
const created = ref<Server | null>(null)
const operation = ref<Operation | null>(null)
const warnings = ref<ServerWarning[]>([])
const cancelling = ref(false)
const retrying = ref(false)
const uploading = ref(false)
const uploadProgress = ref(0)
const hostState = ref<HostState>('loading')
const hostAssignable = ref<number | null>(null)

let socket: ServerSocket | null = null
let poller: ReturnType<typeof setInterval> | null = null

const blocked = computed(() => {
	switch (user.value?.capabilities.blocked_reason) {
		case 'over_limit':
			return formatMessage(messages.overLimit)
		case 'system_user_not_ready':
			return formatMessage(messages.systemUserNotReady)
		default:
			return null
	}
})

const unlimited = computed(() => user.value?.usage.memory.limit_mib === null)

const budgetLeft = computed(() => {
	const usage = user.value?.usage.memory
	if (!usage || usage.limit_mib === null) return 0
	return Math.max(0, usage.limit_mib - usage.allocated_mib)
})

const capacity = computed(() =>
	memoryCeiling(user.value?.usage.memory ?? { limit_mib: 0, allocated_mib: 0, used_bytes: 0 }, {
		state: hostState.value,
		assignableMib: hostAssignable.value,
	}),
)

const ceiling = computed<number | null>(() => capacity.value.max)

const submittable = computed(
	() =>
		!busy.value &&
		name.value.trim().length > 0 &&
		eula.value &&
		ceiling.value !== null &&
		ceiling.value >= MEMORY_MIN,
)

const cancellable = computed(() => stage.value === 'working' && operation.value?.cancellable === true)

const phaseLabel = computed(() => {
	if (uploading.value) return messages.uploading
	return operation.value?.phase ? phaseMessages[operation.value.phase] : messages.working
})

const resultDescription = computed(() => {
	if (stage.value === 'cancelled') return formatMessage(messages.cancelled)
	if (stage.value === 'failed') return formatMessage(messages.failedDescription)
	return formatMessage(messages.working)
})

function reason(error: unknown, table: Record<string, MessageDescriptor> = {}): string {
	if (!isApiRequestError(error)) return formatMessage(messages.unreachable)
	const known = table[error.code]
	return known ? formatMessage(known) : error.message || formatMessage(messages.unreachable)
}

watch(ceiling, (limit) => {
	if (limit === null || limit < MEMORY_MIN) return
	memory.value = Math.min(Math.max(memory.value, MEMORY_MIN), limit)
})

function openSetup(): void {
	if (!submittable.value) return
	submitFailure.value = null
	void setup.value?.show()
}

function browseModpacks(): void {
	keepDraft({ name: name.value, memory: memory.value, eula: eula.value })
	void router.push({ name: 'server-new-modpacks' })
}

function contentOf(choice: SetupChoice): CreateServerContent {
	switch (choice.kind) {
		case 'loader':
			return {
				kind: 'loader',
				loader: choice.loader,
				game_version: choice.gameVersion,
				loader_version: choice.loaderVersion,
			}
		case 'modpack':
			return { kind: 'modpack_project', project_id: choice.projectId, version_id: choice.versionId }
		case 'upload':
			return { kind: 'modpack_upload', file_name: choice.file.name, file_size: choice.file.size }
	}
}

async function create(choice: SetupChoice): Promise<boolean> {
	if (!submittable.value) return false
	busy.value = true
	submitFailure.value = null

	try {
		const response = await api.servers.create({
			name: name.value.trim(),
			owner_id: null,
			memory_mib: memory.value,
			port: null,
			eula_accepted: true,
			content: contentOf(choice),
			properties: choice.properties,
		})
		created.value = response.server
		warnings.value = response.warnings ?? []
		stage.value = 'working'
		follow(response.server.id, response.operation.id)
		apply(response.operation)
		void refresh()
		if (choice.kind === 'upload') void deliver(response.server.id, response.operation.id, choice.file)
		return true
	} catch (error) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.createFailed),
			text: reason(error, submitFailures),
		})
		return false
	} finally {
		busy.value = false
	}
}

async function deliver(serverId: Ulid, operationId: Ulid, file: File): Promise<void> {
	uploading.value = true
	uploadProgress.value = 0
	try {
		const handle = api.operations
			.putPayload(serverId, operationId, file)
			.onProgress((progress) => (uploadProgress.value = progress.progress))
		apply((await handle.promise).operation)
	} catch (error) {
		submitFailure.value = formatMessage(messages.uploadFailed, {
			reason: reason(error, submitFailures),
		})
	} finally {
		uploading.value = false
	}
}

function follow(serverId: Ulid, operationId: Ulid): void {
	unfollow()
	socket = openServerSocket(serverId)
	socket.on('operations', (message) => {
		const found = message.operations.find((entry) => entry.id === operationId)
		if (found) apply(found)
	})
	poller = setInterval(() => {
		if (socket?.status.phase === 'open') return
		api.operations.get(serverId, operationId).then(apply, () => undefined)
	}, POLL_INTERVAL_MS)
}

function unfollow(): void {
	socket?.close()
	socket = null
	if (poller !== null) clearInterval(poller)
	poller = null
}

function apply(next: Operation): void {
	operation.value = next
	if (next.state === 'done') {
		unfollow()
		void refresh()
		void router.replace({ name: 'server-overview', params: { id: next.server_id } })
		return
	}
	if (next.state === 'failed') {
		unfollow()
		stage.value = 'failed'
		return
	}
	if (next.state === 'cancelled') {
		unfollow()
		stage.value = 'cancelled'
		void refresh()
	}
}

async function cancel(): Promise<void> {
	if (!created.value || !operation.value || cancelling.value) return
	cancelling.value = true
	try {
		apply(await api.operations.cancel(created.value.id, operation.value.id))
	} catch (error) {
		submitFailure.value = reason(error)
	} finally {
		cancelling.value = false
	}
}

async function retry(): Promise<void> {
	if (!created.value || !operation.value || retrying.value) return
	retrying.value = true
	try {
		const accepted = await api.operations.retry(created.value.id, operation.value.id)
		stage.value = 'working'
		submitFailure.value = null
		follow(created.value.id, accepted.operation.id)
		apply(accepted.operation)
	} catch (error) {
		submitFailure.value = reason(error)
	} finally {
		retrying.value = false
	}
}

async function loadHost(): Promise<void> {
	hostState.value = 'loading'
	try {
		hostAssignable.value = (await api.admin.host()).assignable_memory_mib
		hostState.value = 'ready'
	} catch {
		hostAssignable.value = null
		hostState.value = 'unavailable'
	}
}

onMounted(() => {
	const draft = takeDraft()
	memory.value = unlimited.value
		? MEMORY_DEFAULT
		: Math.min(MEMORY_DEFAULT, Math.max(MEMORY_MIN, budgetLeft.value))
	if (draft) {
		name.value = draft.name
		memory.value = draft.memory
		eula.value = draft.eula
	}
	if (isAdmin.value) void loadHost()
	else hostState.value = 'unavailable'

	const pick = takeModpackPick()
	if (pick) {
		if (submittable.value || (draft && draft.eula && draft.name.trim())) {
			void setup.value?.showWithModpack(pick)
		}
	}
})

onBeforeUnmount(unfollow)
</script>
