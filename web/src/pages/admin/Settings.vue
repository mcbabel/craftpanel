<template>
	<div class="flex flex-col gap-6">
		<div class="flex flex-col gap-1">
			<h1 class="m-0 text-2xl font-extrabold text-contrast">
				{{ formatMessage(messages.title) }}
			</h1>
			<p class="m-0 text-secondary">{{ formatMessage(messages.subtitle) }}</p>
		</div>

		<LoadingIndicator v-if="loading" />

		<Admonition
			v-else-if="loadFailure"
			type="critical"
			:header="formatMessage(messages.loadFailed)"
			:body="loadFailure"
		>
			<template #actions>
				<Button @click="load()">
					<UpdatedIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.retryButton) }}
				</Button>
			</template>
		</Admonition>

		<template v-else>
			<Admonition v-if="saveFailure" type="critical" :body="saveFailure" />
			<Admonition
				v-else-if="saved"
				type="success"
				:body="formatMessage(commonMessages.changesSavedLabel)"
			/>

			<BrandingSettings />

			<Card class="!mb-0 flex flex-col gap-6">
				<SettingsLabel
					:title="formatMessage(messages.networkTitle)"
					:description="formatMessage(messages.networkDescription)"
				/>

				<div class="flex flex-col gap-1.5">
					<label class="text-sm font-semibold text-contrast" for="public-address">
						{{ formatMessage(messages.publicAddress) }}
					</label>
					<Input
						id="public-address"
						:model-value="draft.public_address ?? ''"
						autocapitalize="none"
						autocorrect="off"
						:spellcheck="false"
						:placeholder="formatMessage(messages.publicAddressPlaceholder)"
						wrapper-class="w-full sm:w-96"
						@update:model-value="setPublicAddress"
					/>
					<span class="text-xs text-secondary">
						{{ formatMessage(messages.publicAddressHint) }}
					</span>
					<Admonition
						v-if="draft.public_address === null"
						type="info"
						:body="formatMessage(messages.publicAddressEmpty)"
					/>
				</div>
			</Card>

			<Card class="!mb-0 flex flex-col gap-6">
				<SettingsLabel
					:title="formatMessage(messages.defaultsTitle)"
					:description="formatMessage(messages.defaultsDescription)"
				/>
				<UserLimitFields
					v-model="draft.default_limits"
					scope="defaults"
					:memory-max="host?.assignable_memory_mib ?? draft.default_limits.memory_mib"
					:disk-max="host?.assignable_disk_mib ?? draft.default_limits.disk_mib"
				/>
			</Card>

			<Card class="!mb-0 flex flex-col gap-6">
				<SettingsLabel
					:title="formatMessage(messages.ceilingsTitle)"
					:description="formatMessage(messages.ceilingsDescription)"
				/>

				<div class="flex flex-col gap-1.5">
					<label class="text-sm font-semibold text-contrast" for="max-upload">
						{{ formatMessage(messages.maxUpload) }}
					</label>
					<div class="flex items-center gap-2">
						<Input
							id="max-upload"
							:model-value="uploadMib"
							type="number"
							:min="1"
							:error="draft.max_upload_bytes < MIB"
							wrapper-class="w-32"
							@update:model-value="setUploadMib"
						/>
						<span class="text-secondary">MiB</span>
					</div>
					<span class="text-xs text-secondary">
						{{
							formatMessage(messages.maxUploadHint, { size: formatBytes(draft.max_upload_bytes) })
						}}
					</span>
				</div>

				<div class="flex flex-col gap-1.5">
					<label class="text-sm font-semibold text-contrast" for="max-backups">
						{{ formatMessage(messages.maxBackups) }}
					</label>
					<Input
						id="max-backups"
						:model-value="draft.max_backups_per_server"
						type="number"
						:min="1"
						:error="draft.max_backups_per_server < 1"
						wrapper-class="w-32"
						@update:model-value="setMaxBackups"
					/>
					<span class="text-xs text-secondary">{{ formatMessage(messages.maxBackupsHint) }}</span>
				</div>

				<div class="flex flex-col gap-1.5">
					<label class="text-sm font-semibold text-contrast" for="max-operations">
						{{ formatMessage(messages.maxOperations) }}
					</label>
					<Input
						id="max-operations"
						:model-value="draft.max_concurrent_operations"
						type="number"
						:min="1"
						:error="draft.max_concurrent_operations < 1"
						wrapper-class="w-32"
						@update:model-value="setMaxOperations"
					/>
					<span class="text-xs text-secondary">
						{{ formatMessage(messages.maxOperationsHint) }}
					</span>
				</div>

				<div class="flex flex-col gap-1.5">
					<label class="text-sm font-semibold text-contrast" for="stop-grace">
						{{ formatMessage(messages.stopGrace) }}
					</label>
					<div class="flex items-center gap-2">
						<Input
							id="stop-grace"
							:model-value="draft.stop_grace_seconds"
							type="number"
							:min="1"
							:error="draft.stop_grace_seconds < 1"
							wrapper-class="w-32"
							@update:model-value="setStopGrace"
						/>
						<span class="text-secondary">{{ formatMessage(messages.seconds) }}</span>
					</div>
					<span class="text-xs text-secondary">{{ formatMessage(messages.stopGraceHint) }}</span>
				</div>
			</Card>

			<Card class="!mb-0 flex flex-col gap-4">
				<div class="flex items-center justify-between gap-4">
					<label class="flex flex-col gap-0.5" for="external-services">
						<span class="text-lg font-extrabold text-contrast">
							{{ formatMessage(messages.externalTitle) }}
						</span>
						<span class="text-sm text-secondary">
							{{ formatMessage(messages.externalDescription) }}
						</span>
					</label>
					<Toggle id="external-services" v-model="draft.external_services_enabled" />
				</div>
				<Admonition
					v-if="!draft.external_services_enabled"
					type="warning"
					:body="formatMessage(messages.externalOff)"
				/>
			</Card>

			<component
				:is="section.component"
				v-for="section of panelSettingsSections"
				:key="section.id"
				v-model="draft"
			/>

			<UnsavedChangesPopup
				:original="original"
				:modified="draft"
				:saving="saving"
				:can-save="valid"
				:save-disabled-reason="valid ? undefined : formatMessage(messages.invalidValues)"
				@save="save"
				@reset="reset"
			/>
		</template>
	</div>
</template>

<script setup lang="ts">
import { UpdatedIcon } from '@modrinth/assets'
import {
	Admonition,
	Button,
	Card,
	commonMessages,
	defineMessages,
	Input,
	LoadingIndicator,
	SettingsLabel,
	Toggle,
	UnsavedChangesPopup,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, toRaw } from 'vue'

import { api, type HostCapacity, isApiRequestError, type PanelSettings } from '@/api'
import { blankSettings } from '@/pages/admin/settings/blank'
import BrandingSettings from '@/pages/admin/settings/Branding.vue'
import { panelSettingsSections, sectionsValid } from '@/pages/admin/settings/sections'
import UserLimitFields from '@/pages/admin/UserLimitFields.vue'

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

const MIB = 1024 * 1024
const MEMORY_MIN = 512
const PIDS_MIN = 64
const DISK_MIN = 1024

const messages = defineMessages({
	title: { id: 'admin.settings.title', defaultMessage: 'Panel settings' },
	subtitle: {
		id: 'admin.settings.subtitle',
		defaultMessage: 'The rules that hold for the whole panel, not for a single server.',
	},
	loadFailed: {
		id: 'admin.settings.load-failed',
		defaultMessage: 'Could not load the panel settings',
	},
	networkTitle: { id: 'admin.settings.network.title', defaultMessage: 'Network' },
	networkDescription: {
		id: 'admin.settings.network.description',
		defaultMessage: 'The address players connect to, and the ports servers may be given.',
	},
	publicAddress: { id: 'admin.settings.public-address', defaultMessage: 'Public address' },
	publicAddressPlaceholder: {
		id: 'admin.settings.public-address.placeholder',
		defaultMessage: 'play.example.com',
	},
	publicAddressHint: {
		id: 'admin.settings.public-address.hint',
		defaultMessage:
			'Shown with the port of every server that has no address of its own. Leave it empty and a server shows only what is set on the server itself.',
	},
	publicAddressEmpty: {
		id: 'admin.settings.public-address.empty',
		defaultMessage: 'No address is set, so servers show only an address set on the server itself.',
	},
	defaultsTitle: {
		id: 'admin.settings.defaults.title',
		defaultMessage: 'Defaults for new accounts',
	},
	defaultsDescription: {
		id: 'admin.settings.defaults.description',
		defaultMessage:
			'What a new account gets when no limits are given at creation. Changing this leaves existing accounts alone.',
	},
	ceilingsTitle: { id: 'admin.settings.ceilings.title', defaultMessage: 'Ceilings' },
	ceilingsDescription: {
		id: 'admin.settings.ceilings.description',
		defaultMessage: 'Bounds the panel enforces on everyone, admins included.',
	},
	maxUpload: { id: 'admin.settings.max-upload', defaultMessage: 'Largest upload' },
	maxUploadHint: {
		id: 'admin.settings.max-upload.hint',
		defaultMessage: 'Holds for file writes and content uploads alike. Currently {size}.',
	},
	maxBackups: { id: 'admin.settings.max-backups', defaultMessage: 'Backups per server' },
	maxBackupsHint: {
		id: 'admin.settings.max-backups.hint',
		defaultMessage: 'Once the count is reached, a new backup is refused until one is deleted.',
	},
	maxOperations: {
		id: 'admin.settings.max-operations',
		defaultMessage: 'Operations at the same time',
	},
	maxOperationsHint: {
		id: 'admin.settings.max-operations.hint',
		defaultMessage: 'Installs, backups and extractions queue up behind this number.',
	},
	stopGrace: { id: 'admin.settings.stop-grace', defaultMessage: 'Grace period when stopping' },
	stopGraceHint: {
		id: 'admin.settings.stop-grace.hint',
		defaultMessage: 'How long a server may take to shut down before it is killed.',
	},
	seconds: { id: 'admin.settings.seconds', defaultMessage: 'seconds' },
	externalTitle: { id: 'admin.settings.external.title', defaultMessage: 'Outbound services' },
	externalDescription: {
		id: 'admin.settings.external.description',
		defaultMessage: 'Lets the panel reach Modrinth for content, and the crash log service.',
	},
	externalOff: {
		id: 'admin.settings.external.off',
		defaultMessage:
			'With this off, browsing and installing content is unavailable and crash reports cannot be shared. Servers keep running.',
	},
	invalidValues: {
		id: 'admin.settings.invalid',
		defaultMessage: 'Some values are out of range. Fix the fields marked in red.',
	},
	unknownError: {
		id: 'admin.settings.unknown-error',
		defaultMessage: 'Something went wrong. Try again.',
	},
})

const draft = ref<PanelSettings>(blankSettings())
const original = ref<PanelSettings>(blankSettings())
const host = ref<HostCapacity | null>(null)
const loading = ref(true)
const saving = ref(false)
const saved = ref(false)
const loadFailure = ref<string | null>(null)
const saveFailure = ref<string | null>(null)

let savedTimer: ReturnType<typeof setTimeout> | undefined

const uploadMib = computed(() => Math.round(draft.value.max_upload_bytes / MIB))

const valid = computed(
	() =>
		draft.value.max_upload_bytes >= MIB &&
		draft.value.max_backups_per_server >= 1 &&
		draft.value.max_concurrent_operations >= 1 &&
		draft.value.stop_grace_seconds >= 1 &&
		draft.value.default_limits.memory_mib >= MEMORY_MIN &&
		draft.value.default_limits.cpu_cores > 0 &&
		draft.value.default_limits.pids_max >= PIDS_MIN &&
		draft.value.default_limits.disk_mib >= DISK_MIN &&
		sectionsValid(draft.value),
)

async function load(): Promise<void> {
	loading.value = true
	loadFailure.value = null
	try {
		const [settings, capacity] = await Promise.all([api.admin.settings(), api.admin.host()])
		adopt(settings)
		host.value = capacity
	} catch (error) {
		loadFailure.value = reason(error)
	} finally {
		loading.value = false
	}
}

onMounted(load)
onUnmounted(() => clearTimeout(savedTimer))

function adopt(settings: PanelSettings): void {
	original.value = structuredClone(settings)
	draft.value = structuredClone(settings)
}

function reset(): void {
	draft.value = structuredClone(toRaw(original.value))
	saveFailure.value = null
}

async function save(): Promise<void> {
	if (saving.value || !valid.value) return
	saving.value = true
	saveFailure.value = null
	try {
		adopt(await api.admin.setSettings(draft.value))
		saved.value = true
		clearTimeout(savedTimer)
		savedTimer = setTimeout(() => {
			saved.value = false
		}, 4000)
	} catch (error) {
		saveFailure.value = reason(error)
	} finally {
		saving.value = false
	}
}

function reason(error: unknown): string {
	return isApiRequestError(error) ? error.message : formatMessage(messages.unknownError)
}

function numberFrom(value: string | number | undefined, fallback: number): number {
	const parsed = typeof value === 'number' ? value : Number(value)
	return Number.isFinite(parsed) ? parsed : fallback
}

function setPublicAddress(value: string | number | undefined): void {
	const text = String(value ?? '').trim()
	draft.value.public_address = text.length > 0 ? text : null
}

function setUploadMib(value: string | number | undefined): void {
	draft.value.max_upload_bytes = Math.max(0, Math.round(numberFrom(value, uploadMib.value))) * MIB
}

function setMaxBackups(value: string | number | undefined): void {
	draft.value.max_backups_per_server = numberFrom(value, draft.value.max_backups_per_server)
}

function setMaxOperations(value: string | number | undefined): void {
	draft.value.max_concurrent_operations = numberFrom(value, draft.value.max_concurrent_operations)
}

function setStopGrace(value: string | number | undefined): void {
	draft.value.stop_grace_seconds = numberFrom(value, draft.value.stop_grace_seconds)
}
</script>
