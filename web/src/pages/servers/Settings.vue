<template>
	<div class="flex flex-col gap-6 max-md:gap-4">
		<div class="isolate flex w-full select-none flex-col gap-4 overflow-auto max-md:hidden">
			<NavTabs :links="tabs" />
		</div>

		<div class="md:hidden">
			<Chips
				:model-value="section"
				:items="visibleSections"
				:format-label="sectionLabel"
				:never-empty="true"
				@update:model-value="openSection"
			/>
		</div>

		<GeneralSection v-if="section === 'general'" />
		<InstallationSection v-else-if="section === 'installation'" />
		<NetworkSection v-else-if="section === 'network'" />
		<ServerSettingsBackupsPage v-else-if="section === 'backups'" />
		<ServerSettingsPropertiesPage v-else-if="section === 'properties'" />
		<SharedLogsSection v-else-if="section === 'shared-logs'" />
		<AdvancedSection v-else />
	</div>
</template>

<script setup lang="ts">
import { ShareIcon } from '@modrinth/assets'
import {
	Chips,
	defineMessage,
	defineMessages,
	injectModrinthServerContext,
	NavTabs,
	useVIntl,
	provideServerSettings,
	ServerSettingsBackupsPage,
	ServerSettingsPropertiesPage,
	type ServerSettingsTabId,
	serverSettingsTabDefinitions,
} from '@modrinth/ui'
import { type Component, computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { useSession } from '@/composables/session'

import AdvancedSection from './settings/Advanced.vue'
import GeneralSection from './settings/General.vue'
import InstallationSection from './settings/Installation.vue'
import NetworkSection from './settings/Network.vue'
import SharedLogsSection from './settings/SharedLogs.vue'

const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()
const { user, isAdmin } = useSession()
const { serverId, server } = injectModrinthServerContext()

// The panel's own sections sit beside the ones the vendored settings layout defines.
type SectionId = ServerSettingsTabId | 'shared-logs'

const SECTIONS: SectionId[] = [
	'general',
	'installation',
	'network',
	'backups',
	'properties',
	'shared-logs',
	'advanced',
]

const messages = defineMessages({
	sharedLogs: { id: 'craftpanel.settings.tabs.shared-logs', defaultMessage: 'Shared logs' },
})

function isSection(value: unknown): value is SectionId {
	return typeof value === 'string' && SECTIONS.includes(value as SectionId)
}

const section = computed<SectionId>(() =>
	isSection(route.params.section) ? route.params.section : 'general',
)

watch(
	() => route.params.section,
	(value) => {
		if (route.name !== 'server-settings' || isSection(value)) return
		void router.replace({ name: 'server-settings', params: { section: 'general' } })
	},
	{ immediate: true },
)

const isProxy = computed(() => server.value?.loader?.toLowerCase() === 'velocity')

const vendorTabs = computed(() =>
	serverSettingsTabDefinitions.filter((tab) => isSection(tab.id)).map((tab) => ({
		label: formatMessage(
			defineMessage({ id: `server.settings.tabs.${tab.id}`, defaultMessage: tab.label }),
		),
		id: tab.id,
		icon: tab.icon,
		href: `/servers/${serverId}/settings/${tab.id}`,
		shown:
			(tab.shown?.({
				serverId,
				ownerId: server.value?.owner_id ?? '',
				serverStatus: server.value?.status,
				isOwner: server.value?.owner_id === user.value?.id,
				isAdmin: isAdmin.value,
			}) ?? true) &&
			(tab.id !== 'properties' || !isProxy.value),
	})),
)

const tabs = computed(() => {
	const sharedLogs = {
		label: formatMessage(messages.sharedLogs),
		id: 'shared-logs' as SectionId,
		icon: ShareIcon,
		href: `/servers/${serverId}/settings/shared-logs`,
		shown: true,
	}
	const list: { label: string; id: SectionId; icon: Component; href: string; shown: boolean }[] = [
		...vendorTabs.value,
	]
	const before = list.findIndex((tab) => tab.id === 'advanced')
	list.splice(before === -1 ? list.length : before, 0, sharedLogs)
	return list
})

const visibleSections = computed(() =>
	tabs.value.filter((tab) => tab.shown).map((tab) => tab.id),
)

function sectionLabel(id: SectionId): string {
	return tabs.value.find((tab) => tab.id === id)?.label ?? id
}

function openSection(id: SectionId | null | undefined): void {
	if (!id || id === section.value) return
	void router.replace({ name: 'server-settings', params: { section: id } })
}

provideServerSettings({
	isApp: ref(false),
	currentUserId: computed(() => user.value?.id ?? null),
	currentUserRole: computed(() => user.value?.panel_role ?? null),
	browseModpacks: () => {
		void router.push({ name: 'server-browse', params: { id: serverId }, query: { type: 'modpack' } })
	},
})
</script>
