<template>
	<CreationFlowModal
		v-if="catalog.ready.value"
		ref="flow"
		:type="type"
		:available-loaders="catalog.flowLoaders.value"
		:show-snapshot-toggle="true"
		:initial-loader="initialLoader"
		:initial-game-version="initialGameVersion"
		:fade="type === 'reset-server' ? 'danger' : undefined"
		:search-projects="searchModpacks"
		:get-project-versions="getProjectVersions"
		:get-loader-manifest="catalog.manifest"
		:finish-disabled="finishDisabled"
		:finish-disabled-tooltip="finishDisabledTooltip"
		@create="onCreate"
		@browse-modpacks="emit('browse-modpacks')"
	/>
</template>

<script setup lang="ts">
import type { AbstractModrinthClient, Labrinth } from '@modrinth/api-client'
import {
	defineMessages,
	injectModrinthClient,
	injectNotificationManager,
	injectTags,
	provideModrinthClient,
	provideTags,
	useVIntl,
} from '@modrinth/ui'
import type {
	CreationFlowContextValue,
	ProjectSearchResult,
} from '@modrinth/ui/src/components/flows/creation-flow-modal/creation-flow-context.ts'
import CreationFlowModal from '@modrinth/ui/src/components/flows/creation-flow-modal/index.vue'
import { nextTick, ref, useTemplateRef, watch } from 'vue'

import type { LoaderId, PropertiesFields } from '@/api'
import {
	type ModpackPick,
	modpackFilter,
	runnableVersions,
	type SetupChoice,
	useCreationCatalog,
} from '@/providers/creation-catalog'

const props = withDefaults(
	defineProps<{
		type?: 'server-onboarding' | 'reset-server'
		initialLoader?: string
		initialGameVersion?: string
		finishDisabled?: boolean
		finishDisabledTooltip?: string
		submit: (choice: SetupChoice) => Promise<boolean>
	}>(),
	{
		type: 'server-onboarding',
		initialLoader: undefined,
		initialGameVersion: undefined,
		finishDisabled: false,
		finishDisabledTooltip: undefined,
	},
)

const emit = defineEmits<{ 'browse-modpacks': [] }>()

const messages = defineMessages({
	catalogFailed: {
		id: 'craftpanel.setup.catalog-failed',
		defaultMessage: 'The loader catalogue could not be loaded',
	},
})

const { formatMessage } = useVIntl()
const real = injectModrinthClient()
const { addNotification, handleError } = injectNotificationManager()
const catalog = useCreationCatalog()

const shim: AbstractModrinthClient = Object.create(real)
Object.defineProperty(shim, 'paper', { value: catalog.paper, enumerable: true })
Object.defineProperty(shim, 'purpur', { value: catalog.purpur, enumerable: true })
provideModrinthClient(shim)

const versionTags = ref<Labrinth.Tags.v2.GameVersion[]>([])
watch(catalog.gameVersionTags, (tags) => (versionTags.value = tags), { immediate: true })
provideTags({ gameVersions: versionTags, loaders: injectTags(null)?.loaders ?? ref([]) })

const flow = useTemplateRef<InstanceType<typeof CreationFlowModal>>('flow')

let loading: Promise<void> | null = null

function ensureCatalog(): Promise<void> {
	if (catalog.ready.value) return Promise.resolve()
	loading ??= catalog.load().finally(() => {
		loading = null
	})
	return loading
}

async function searchModpacks(query: string, limit = 10): Promise<ProjectSearchResult> {
	const loaders = catalog.modpackLoaders.value
	if (loaders.length === 0) return { hits: [], total_hits: 0, offset: 0, limit }
	const found = await real.labrinth.projects_v2.search({
		query: query || undefined,
		new_filters: modpackFilter(loaders),
		limit,
	})
	return found as unknown as ProjectSearchResult
}

async function runnable(projectId: string): Promise<Labrinth.Versions.v2.Version[]> {
	const versions = await real.labrinth.versions_v2.getProjectVersions(projectId, {
		include_changelog: false,
	})
	return runnableVersions(versions, catalog.modpackLoaders.value)
}

async function getProjectVersions(projectId: string): Promise<{ id: string }[]> {
	return (await runnable(projectId)).map((version) => ({ id: version.id }))
}

function choiceOf(ctx: CreationFlowContextValue): SetupChoice | null {
	const properties = ctx.buildProperties() as PropertiesFields
	if (ctx.setupType.value === 'modpack') {
		if (ctx.modpackFile.value) return { kind: 'upload', file: ctx.modpackFile.value, properties }
		const selection = ctx.modpackSelection.value
		if (!selection) return null
		return {
			kind: 'modpack',
			projectId: selection.projectId,
			versionId: selection.versionId,
			name: selection.name,
			properties,
		}
	}
	const gameVersion = ctx.selectedGameVersion.value
	if (!gameVersion) return null
	const loader = (
		ctx.setupType.value === 'vanilla' ? 'vanilla' : (ctx.selectedLoader.value ?? 'vanilla')
	) as LoaderId
	return {
		kind: 'loader',
		loader,
		gameVersion,
		loaderVersion: loader === 'vanilla' ? null : ctx.selectedLoaderVersion.value,
		properties,
	}
}

async function onCreate(ctx: CreationFlowContextValue): Promise<void> {
	const choice = choiceOf(ctx)
	try {
		if (choice && (await props.submit(choice))) flow.value?.hide()
	} catch (cause) {
		handleError(cause instanceof Error ? cause : new Error(String(cause)))
	} finally {
		ctx.loading.value = false
	}
}

async function show(): Promise<void> {
	await ensureCatalog()
	if (!catalog.ready.value) {
		addNotification({
			type: 'error',
			title: formatMessage(messages.catalogFailed),
			text: catalog.failure.value instanceof Error ? catalog.failure.value.message : undefined,
		})
		return
	}
	await nextTick()
	await flow.value?.show()
}

async function showWithModpack(pick: ModpackPick): Promise<void> {
	await show()
	const ctx = flow.value?.ctx
	if (!ctx) return
	try {
		const versionId = pick.versionId ?? (await runnable(pick.projectId))[0]?.id
		if (!versionId) return
		ctx.setSetupType('modpack')
		ctx.modpackSelection.value = {
			projectId: pick.projectId,
			versionId,
			name: pick.name,
			iconUrl: pick.iconUrl,
		}
		ctx.modal.value?.setStage('final-config')
	} catch (cause) {
		handleError(cause instanceof Error ? cause : new Error(String(cause)))
	}
}

function hide(): void {
	flow.value?.hide()
}

defineExpose({ show, showWithModpack, hide, catalog })
</script>
