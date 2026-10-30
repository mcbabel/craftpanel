<template>
	<NewModal ref="modal" :header="formatMessage(messages.title)" width="560px">
		<form class="flex w-full flex-col gap-3" @submit.prevent="() => void confirm()">
			<span>{{ formatMessage(messages.body, { file: target?.fileName ?? '' }) }}</span>
			<ProjectCombobox
				v-model="projectId"
				:project-types="projectTypes"
				:limit="20"
				clearable
			/>
			<template v-if="projectId">
				<label class="flex flex-col gap-1.5">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.gameVersion) }}</span>
					<Combobox
						v-model="gameVersion"
						:options="gameVersionOptions"
						:disabled="versionsLoading"
						searchable
					/>
				</label>
				<label class="flex flex-col gap-1.5">
					<span class="font-semibold text-contrast">{{ formatMessage(messages.version) }}</span>
					<Combobox
						v-model="versionId"
						:options="versionOptions"
						:disabled="versionsLoading"
						:no-options-message="formatMessage(messages.noVersions)"
						searchable
					/>
				</label>
			</template>
			<span class="text-sm text-secondary">{{ formatMessage(messages.hint) }}</span>
			<div class="mb-1 mt-2 flex justify-end gap-2.5">
				<Button @click="modal?.hide()">
					<XIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button type="colored" color="brand" native-type="submit" :disabled="!projectId || busy">
					<LinkIcon aria-hidden="true" />
					{{ formatMessage(messages.confirm) }}
				</Button>
			</div>
		</form>
	</NewModal>
</template>

<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import { LinkIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	Combobox,
	type ComboboxOption,
	commonMessages,
	defineMessages,
	injectModrinthClient,
	injectNotificationManager,
	NewModal,
	ProjectCombobox,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { api, type Ulid } from '@/api'
import type { LinkTarget } from '@/providers/content-manager'

const props = defineProps<{ serverId: Ulid; serverGameVersion?: string; serverLoader?: string }>()

const AUTOMATIC = 'automatic'
const EVERY = 'every'
const PLUGIN_LOADERS = ['bukkit', 'spigot', 'paper', 'purpur', 'folia']

const { formatMessage } = useVIntl()
const { addNotification } = injectNotificationManager()
const { labrinth } = injectModrinthClient()

const modal = ref<InstanceType<typeof NewModal>>()
const target = ref<LinkTarget | null>(null)
const projectId = ref<string | undefined>(undefined)
const busy = ref(false)

const versions = ref<Labrinth.Versions.v2.Version[]>([])
const versionsLoading = ref(false)
const gameVersion = ref<string>(EVERY)
const versionId = ref<string>(AUTOMATIC)
let asked = 0

const projectTypes = computed(() => {
	const kind = target.value?.projectType
	if (kind === 'plugin') return ['plugin' as const]
	if (kind === 'mod') return ['mod' as const]
	return kind ? [kind] : undefined
})

const messages = defineMessages({
	title: { id: 'craftpanel.content.link.title', defaultMessage: 'Link to a Modrinth project' },
	body: {
		id: 'craftpanel.content.link.body',
		defaultMessage: 'Modrinth did not recognise {file}. Pick the project it belongs to.',
	},
	hint: {
		id: 'craftpanel.content.link.hint',
		defaultMessage:
			'The file stays as it is. Left on automatic, the panel takes the version that matches its name. Updates are checked from then on.',
	},
	gameVersion: { id: 'craftpanel.content.link.game-version', defaultMessage: 'Game version' },
	everyGameVersion: {
		id: 'craftpanel.content.link.every-game-version',
		defaultMessage: 'All game versions',
	},
	version: { id: 'craftpanel.content.link.version', defaultMessage: 'Version of the file' },
	automatic: {
		id: 'craftpanel.content.link.automatic',
		defaultMessage: 'Automatic, from the file name',
	},
	noVersions: {
		id: 'craftpanel.content.link.no-versions',
		defaultMessage: 'No version for this game version',
	},
	versionsFailed: {
		id: 'craftpanel.content.link.versions-failed',
		defaultMessage: 'The versions of this project could not be loaded',
	},
	confirm: { id: 'craftpanel.content.link.confirm', defaultMessage: 'Link' },
	linked: { id: 'craftpanel.content.link.linked', defaultMessage: 'Linked to Modrinth' },
	linkedNoVersion: {
		id: 'craftpanel.content.link.linked-no-version',
		defaultMessage:
			'Linked to Modrinth. No version matched the file name, so pick one under Update to replace the file.',
	},
	failed: { id: 'craftpanel.content.link.failed', defaultMessage: 'The file could not be linked' },
})

function runsHere(version: Labrinth.Versions.v2.Version): boolean {
	if (target.value?.projectType === 'plugin') {
		return version.loaders.some((loader) => PLUGIN_LOADERS.includes(loader))
	}
	const own = props.serverLoader
	if (!own) return true
	return version.loaders.includes(own) || (own === 'quilt' && version.loaders.includes('fabric'))
}

const usable = computed(() => {
	const fitting = versions.value.filter(runsHere)
	return fitting.length ? fitting : versions.value
})

const gameVersionOptions = computed<ComboboxOption<string>[]>(() => {
	const seen = new Set<string>()
	for (const version of usable.value) {
		for (const game of version.game_versions) seen.add(game)
	}
	return [
		{ label: formatMessage(messages.everyGameVersion), value: EVERY },
		...[...seen].map((game) => ({ label: game, value: game })),
	]
})

const versionOptions = computed<ComboboxOption<string>[]>(() => {
	const fitting = usable.value.filter(
		(version) => gameVersion.value === EVERY || version.game_versions.includes(gameVersion.value),
	)
	return [
		{ label: formatMessage(messages.automatic), value: AUTOMATIC },
		...fitting.map((version) => ({
			label: version.loaders.length
				? `${version.version_number} (${version.loaders.join(', ')})`
				: version.version_number,
			value: version.id,
		})),
	]
})

watch(projectId, async (id) => {
	const turn = ++asked
	versions.value = []
	gameVersion.value = EVERY
	versionId.value = AUTOMATIC
	if (!id) return
	versionsLoading.value = true
	try {
		const found = await labrinth.versions_v2.getProjectVersions(id, { include_changelog: false })
		if (turn !== asked) return
		versions.value = found
		const own = props.serverGameVersion
		if (own && usable.value.some((version) => version.game_versions.includes(own))) {
			gameVersion.value = own
		}
	} catch (cause) {
		if (turn !== asked) return
		addNotification({
			type: 'error',
			title: formatMessage(messages.versionsFailed),
			text: cause instanceof Error ? cause.message : undefined,
		})
	} finally {
		if (turn === asked) versionsLoading.value = false
	}
})

watch(gameVersion, () => {
	if (versionId.value === AUTOMATIC) return
	if (!versionOptions.value.some((option) => option.value === versionId.value)) {
		versionId.value = AUTOMATIC
	}
})

function open(next: LinkTarget): void {
	target.value = next
	projectId.value = undefined
	modal.value?.show()
}

async function confirm(): Promise<void> {
	const item = target.value
	if (!item || !projectId.value) return
	busy.value = true
	try {
		const linked = await api.content.link(props.serverId, {
			id: item.id,
			project_id: projectId.value,
			...(versionId.value === AUTOMATIC ? {} : { version_id: versionId.value }),
		})
		addNotification({
			type: 'success',
			title: formatMessage(linked.version_id ? messages.linked : messages.linkedNoVersion),
		})
		modal.value?.hide()
	} catch (cause) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.failed),
			text: cause instanceof Error ? cause.message : undefined,
		})
	} finally {
		busy.value = false
	}
}

defineExpose({ open })
</script>
