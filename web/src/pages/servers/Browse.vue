<template>
	<div v-if="loading" class="relative grid place-items-center py-16">
		<LoadingIndicator />
	</div>

	<div v-else-if="browse.loadError.value" class="flex justify-center py-16">
		<ErrorInformationCard
			:title="formatMessage(messages.catalogueFailedTitle)"
			:description="formatMessage(messages.catalogueFailedText)"
			:icon="TriangleAlertIcon"
			:action="{
				label: formatMessage(commonMessages.retryButton),
				onClick: () => void browse.load(),
				icon: UpdatedIcon,
			}"
			:error-details="[
				{
					label: formatMessage(commonMessages.errorLabel),
					value: browse.loadError.value.message,
					type: 'inline',
				},
			]"
		/>
	</div>

	<div v-else class="flex flex-col gap-4">
		<BrowseInstallHeader divider bottom-padding />

		<Admonition
			v-if="browse.searchError.value"
			type="critical"
			:header="formatMessage(messages.searchFailedTitle)"
			:body="browse.searchError.value.message"
		>
			<template #actions>
				<Button type="colored" color="red" @click="browse.refreshSearch()">
					<UpdatedIcon />
					{{ formatMessage(commonMessages.retryButton) }}
				</Button>
			</template>
		</Admonition>

		<div class="grid min-w-0 gap-3 lg:grid-cols-[18.75rem_minmax(0,1fr)]">
			<aside class="min-w-0" :aria-label="formatMessage(commonMessages.filtersLabel)">
				<BrowseSidebar />
			</aside>

			<section class="flex min-w-0 flex-col gap-3">
				<BrowsePageLayout>
					<template #display-mode-icon>
						<GridIcon v-if="browse.displayMode.value === 'grid'" />
						<ImageIcon v-else-if="browse.displayMode.value === 'gallery'" />
						<ListIcon v-else />
					</template>
				</BrowsePageLayout>
			</section>
		</div>

		<SelectedProjectsFloatingBar />
	</div>
</template>

<script setup lang="ts">
import { GridIcon, ImageIcon, ListIcon, TriangleAlertIcon, UpdatedIcon } from '@modrinth/assets'
import {
	Admonition,
	BrowseInstallHeader,
	BrowsePageLayout,
	BrowseSidebar,
	Button,
	commonMessages,
	defineMessages,
	ErrorInformationCard,
	LoadingIndicator,
	SelectedProjectsFloatingBar,
	useVIntl,
} from '@modrinth/ui'
import { computed } from 'vue'
import { type HistoryState, type RouteLocationRaw, useRoute } from 'vue-router'

import { useServerPage } from '@/composables/server-page'
import { type BrowseManagerOptions, useBrowseManager } from '@/providers/browse-manager'
import { modpackState } from '@/providers/creation-catalog'

const messages = defineMessages({
	catalogueFailedTitle: {
		id: 'craftpanel.browse.catalogue-failed-title',
		defaultMessage: 'The catalogue could not be opened',
	},
	catalogueFailedText: {
		id: 'craftpanel.browse.catalogue-failed-text',
		defaultMessage:
			'Modrinth is fetched by the panel, not by your browser. This machine did not reach it.',
	},
	searchFailedTitle: {
		id: 'craftpanel.browse.search-failed-title',
		defaultMessage: 'The search failed',
	},
})

const { formatMessage } = useVIntl()
const route = useRoute()

function browseOptions(): BrowseManagerOptions {
	if (route.name === 'server-new-modpacks') {
		return {
			serverId: null,
			back: { name: 'server-new' },
			modpacks: { pick: (pick) => ({ name: 'server-new', state: modpackState(pick) }) },
		}
	}

	const page = useServerPage()
	const common = {
		serverId: page.serverId,
		socket: page.socket,
		busyReasons: page.context.busyReasons,
	}
	if (route.query.type !== 'modpack') {
		return { ...common, back: { name: 'server-content', params: { id: page.serverId } } }
	}

	const installation = (state?: HistoryState): RouteLocationRaw => ({
		name: 'server-settings',
		params: { id: page.serverId, section: 'installation' },
		state,
	})
	const busyReasons = page.context.busyReasons
	return {
		...common,
		back: installation(),
		modpacks: {
			pick: (pick) => installation(modpackState(pick)),
			busy: computed(() =>
				busyReasons.value.length > 0 ? formatMessage(busyReasons.value[0].reason) : null,
			),
		},
	}
}

const browse = useBrowseManager(browseOptions())

const loading = computed(() => !browse.ready.value && browse.loadError.value === null)
</script>
