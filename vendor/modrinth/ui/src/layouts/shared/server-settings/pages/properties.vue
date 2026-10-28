<template>
	<div class="relative h-screen w-full select-none max-h-[min(70vh,750px)]">
		<div v-if="propsData" class="flex h-full w-full flex-col justify-between gap-4">
			<div class="flex flex-col gap-4">
				<Admonition
					v-if="hasNoProperties"
					type="warning"
					:body="formatMessage(messages.missingProperties)"
				/>
				<div class="flex flex-col gap-2">
					<div class="m-0">
						{{ formatMessage(messages.introBeforeFilesTab) }}
						<AutoLink
							class="goto-link !inline-block"
							:to="filesTabLink"
							@click="onFilesTabLinkClick"
						>
							{{ formatMessage(messages.filesTab) }}
						</AutoLink>
						{{ formatMessage(messages.introBeforeWiki) }}
						<AutoLink
							class="goto-link !inline-block"
							to="https://minecraft.wiki/w/Server.properties"
							target="_blank"
						>
							{{ formatMessage(messages.minecraftWiki) }}
						</AutoLink>
						{{ formatMessage(messages.introAfterWiki) }}
					</div>
				</div>

				<div class="w-full text-sm">
					<label for="search-server-properties" class="sr-only">
						{{ formatMessage(messages.searchLabel) }}
					</label>
					<Input
						id="search-server-properties"
						v-model="searchInput"
						wrapper-class="w-full"
						type="search"
						:icon="SearchIcon"
						name="search"
						autocomplete="off"
						:placeholder="formatMessage(messages.searchPlaceholder)"
					/>
				</div>
				<div class="flex flex-col gap-3 pb-2">
					<div class="flex flex-col gap-6">
						<!-- Basic Properties -->
						<!-- [&:not(:has(*:not(:empty)))]:hidden is to hide parent if all children are empty -->
						<div
							class="rounded-2xl border border-solid border-surface-5 p-4 pb-2 [&:not(:has(*:not(:empty)))]:hidden"
						>
							<div class="flex w-full flex-col gap-1.5">
								<div v-if="isPropertyVisible('gamemode')" class="flex flex-col gap-2.5 my-1">
									<span class="font-semibold text-contrast">{{ formatMessage(messages.gamemode) }}</span>
									<Chips
										v-model="combinedGamemode"
										:items="gamemodeItems"
										:format-label="formatOptionLabel"
										:disabled-items="canUseAdvancedSettings ? [] : gamemodeItems"
										:disabled-tooltip="permissionDeniedMessage"
									/>
								</div>

								<div
									v-if="combinedGamemode !== 'hardcore' && isPropertyVisible('difficulty')"
									class="flex flex-col gap-2.5 my-1"
								>
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.difficulty)
									}}</span>
									<Chips
										v-model="selectedDifficulty"
										:items="difficultyItems"
										:format-label="formatOptionLabel"
										:disabled-items="canUseAdvancedSettings ? [] : difficultyItems"
										:disabled-tooltip="permissionDeniedMessage"
									/>
								</div>

								<div v-if="isPropertyVisible('max_players')" class="flex flex-col gap-2.5 my-1">
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.maxPlayers)
									}}</span>
									<Input
										id="server-property-max-players"
										v-tooltip="advancedActionTooltip"
										:model-value="liveProperties.max_players"
										type="number"
										placeholder="20"
										wrapper-class="w-full max-w-[450px]"
										:disabled="!canUseAdvancedSettings"
										@update:model-value="liveProperties.max_players = String($event)"
									/>
								</div>

								<div v-if="isPropertyVisible('motd')" class="flex flex-col gap-2.5 my-1">
									<span class="font-semibold text-contrast">MOTD</span>
									<Input
										id="server-property-motd"
										v-model="liveProperties.motd"
										v-tooltip="advancedActionTooltip"
										:placeholder="formatMessage(messages.motdPlaceholder)"
										wrapper-class="w-full max-w-[450px]"
										:disabled="!canUseAdvancedSettings"
									/>
									<InlineValidationMessage :message="motdValidationMessage" class="text-sm" />
								</div>

								<div
									v-if="isPropertyVisible('allow_flight')"
									class="flex flex-row items-center justify-between gap-4 h-10"
								>
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.allowFlight)
									}}</span>
									<Toggle
										id="server-property-allow-flight"
										v-tooltip="advancedActionTooltip"
										:model-value="liveProperties.allow_flight === 'true'"
										:disabled="!canUseAdvancedSettings"
										@update:model-value="liveProperties.allow_flight = $event ? 'true' : 'false'"
									/>
								</div>

								<div
									v-if="isPropertyVisible('allow_cheats')"
									class="flex flex-row items-center justify-between gap-4 h-10"
								>
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.allowCheats)
									}}</span>
									<Toggle
										id="server-property-allow-cheats"
										v-tooltip="advancedActionTooltip"
										:model-value="liveProperties.allow_cheats === 'true'"
										:disabled="!canUseAdvancedSettings"
										@update:model-value="liveProperties.allow_cheats = $event ? 'true' : 'false'"
									/>
								</div>

								<div
									v-if="isPropertyVisible('white_list')"
									class="flex flex-row items-center justify-between gap-4 h-10"
								>
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.enableWhitelist)
									}}</span>
									<Toggle
										id="server-property-whitelist"
										v-model="whitelistEnabled"
										v-tooltip="advancedActionTooltip"
										:disabled="!canUseAdvancedSettings"
									/>
								</div>

								<div
									v-if="isPropertyVisible('spawn_protection')"
									class="flex flex-row items-center justify-between gap-4 h-10"
								>
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.enableSpawnProtection)
									}}</span>
									<Toggle
										id="server-property-spawn-protection-toggle"
										v-model="spawnProtectionEnabled"
										v-tooltip="advancedActionTooltip"
										:disabled="!canUseAdvancedSettings"
									/>
								</div>

								<div
									v-if="spawnProtectionEnabled && isPropertyVisible('spawn_protection')"
									class="flex items-center justify-between h-10"
								>
									<span class="font-semibold text-contrast">{{
										formatMessage(messages.protectionRadius)
									}}</span>
									<Input
										id="server-property-spawn-protection-radius"
										v-tooltip="advancedActionTooltip"
										:model-value="liveProperties.spawn_protection"
										type="number"
										wrapper-class="w-full sm:w-[100px]"
										input-class="text-right"
										:disabled="!canUseAdvancedSettings"
										@update:model-value="liveProperties.spawn_protection = String($event)"
									/>
								</div>
							</div>
						</div>
					</div>
					<!-- Advanced Properties -->
					<Accordion
						v-if="hasVisibleAdvancedProperties"
						overflow-visible
						:force-open="isSearchActive"
						button-class="flex w-full flex-col gap-2 bg-transparent m-0 p-0 border-none"
					>
						<template #title>
							<span class="text-lg font-semibold text-contrast">{{
								formatMessage(messages.advancedProperties)
							}}</span>
						</template>

						<div class="flex flex-col gap-6 pt-4">
							<template v-for="group in advancedGroupedProperties" :key="group.label">
								<div v-if="hasVisibleProperties(group)" class="flex flex-col gap-2.5">
									<h3 class="m-0 text-base font-semibold text-contrast">
										{{ group.label }}
									</h3>
									<div
										class="flex flex-col gap-2 rounded-2xl border border-solid border-surface-5 p-4"
									>
										<template v-for="key in group.properties" :key="key">
											<div
												v-if="isPropertyVisible(key)"
												class="flex flex-row flex-wrap items-center justify-between h-10"
											>
												<span :id="`property-label-${key}`" class="font-semibold text-contrast">
													{{ formatPropertyName(key) }}
												</span>

												<div
													v-if="getPropertyDef(key).type === 'toggle'"
													class="flex w-full justify-end sm:w-[320px]"
												>
													<Toggle
														:id="`server-property-${key}`"
														v-tooltip="advancedActionTooltip"
														:model-value="liveProperties[key] === 'true'"
														:aria-labelledby="`property-label-${key}`"
														:disabled="!canUseAdvancedSettings"
														@update:model-value="liveProperties[key] = $event ? 'true' : 'false'"
													/>
												</div>
												<div
													v-else-if="getPropertyDef(key).type === 'number'"
													class="w-full sm:w-[320px]"
												>
													<Input
														:id="`server-property-${key}`"
														v-tooltip="advancedActionTooltip"
														:model-value="liveProperties[key]"
														type="number"
														:placeholder="formatMessage(messages.typeHere)"
														wrapper-class="w-full"
														:aria-labelledby="`property-label-${key}`"
														:disabled="!canUseAdvancedSettings"
														@update:model-value="liveProperties[key] = String($event)"
													/>
												</div>
												<div v-else class="flex w-full justify-end sm:w-[320px]">
													<Input
														:id="`server-property-${key}`"
														v-model="liveProperties[key]"
														v-tooltip="advancedActionTooltip"
														:placeholder="formatMessage(messages.typeHere)"
														wrapper-class="w-full"
														:aria-labelledby="`property-label-${key}`"
														:disabled="!canUseAdvancedSettings"
													/>
												</div>
											</div>
										</template>
									</div>
								</div>
							</template>
							<div>
								{{ formatMessage(messages.otherPropertiesBeforeFilesTab) }}
								<AutoLink
									class="goto-link !inline-block"
									:to="filesTabLink"
									@click="onFilesTabLinkClick"
								>
									{{ formatMessage(messages.filesTab) }} </AutoLink
								>.
							</div>
						</div>
					</Accordion>

					<div
						v-if="hasNoResults"
						class="flex flex-col items-center gap-2 py-8 text-center text-secondary"
					>
						<SearchIcon class="size-10" />
						<span class="text-lg font-semibold text-contrast">{{
							formatMessage(messages.noPropertiesFound)
						}}</span>
						<span>{{ formatMessage(messages.noPropertiesMatch, { query: searchInput }) }}</span>
					</div>
				</div>
			</div>
		</div>
		<div v-else class="flex h-full w-full items-center justify-center">
			<SpinnerIcon class="animate-spin" />
		</div>

		<SaveBanner
			:has-errors="motdBlocksSave"
			:is-visible="hasUnsavedChanges || isUpdating"
			:server-id="serverId"
			:is-updating="isUpdating || busyReasons.length > 0"
			:restart="canUsePowerActions"
			:save="saveProperties"
			:reset="resetProperties"
		/>
	</div>
</template>

<script setup lang="ts">
import type { Archon } from '@modrinth/api-client'
import { SearchIcon, SpinnerIcon } from '@modrinth/assets'
import { isStaff } from '@modrinth/utils'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import Fuse from 'fuse.js'
import { computed, ref, watch } from 'vue'

import {
	Accordion,
	Admonition,
	AutoLink,
	Chips,
	InlineValidationMessage,
	Input,
	Toggle,
} from '#ui/components'
import SaveBanner from '#ui/components/servers/SaveBanner.vue'
import { defineMessages, type MessageDescriptor, useVIntl } from '#ui/composables/i18n'
import { useServerPermissions } from '#ui/composables/server-permissions'
import { injectServerSettings } from '#ui/layouts/shared/server-settings'
import {
	injectAuth,
	injectModrinthClient,
	injectModrinthServerContext,
	injectNotificationManager,
} from '#ui/providers'
import { commonMessages } from '#ui/utils/common-messages'
import { validateProfanity } from '#ui/utils/profanity-check'

const { addNotification } = injectNotificationManager()
const auth = injectAuth()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	motdProfanity: {
		id: 'hosting.settings.properties.motd-profanity',
		defaultMessage:
			"Due to Minecraft Usage Guidelines, your server's MOTD cannot contain profanity. Detected: “{value}”.",
	},
	motdProfanitySaveError: {
		id: 'hosting.settings.properties.motd-profanity-save-error',
		defaultMessage:
			"Due to Minecraft Usage Guidelines, your server's MOTD cannot contain profanity. Detected: “{value}”",
	},
	missingProperties: {
		id: 'craftpanel.vendor.properties.missing-properties',
		defaultMessage: "Some expected properties are missing from your server.properties - this usually means the server hasn't completed its first startup yet.",
	},
	introBeforeFilesTab: {
		id: 'craftpanel.vendor.properties.intro-before-files-tab',
		defaultMessage: 'Edit the Minecraft server properties file here, or use the',
	},
	filesTab: {
		id: 'craftpanel.vendor.properties.files-tab',
		defaultMessage: 'Files tab',
	},
	introBeforeWiki: {
		id: 'craftpanel.vendor.properties.intro-before-wiki',
		defaultMessage: "to edit the full file. If you're unsure about a setting, the",
	},
	minecraftWiki: {
		id: 'craftpanel.vendor.properties.minecraft-wiki',
		defaultMessage: 'Minecraft Wiki',
	},
	introAfterWiki: {
		id: 'craftpanel.vendor.properties.intro-after-wiki',
		defaultMessage: 'has more details.',
	},
	searchLabel: {
		id: 'craftpanel.vendor.properties.search-label',
		defaultMessage: 'Search server properties',
	},
	searchPlaceholder: {
		id: 'craftpanel.vendor.properties.search-placeholder',
		defaultMessage: 'Search server properties...',
	},
	gamemode: {
		id: 'craftpanel.vendor.properties.gamemode',
		defaultMessage: 'Gamemode',
	},
	difficulty: {
		id: 'craftpanel.vendor.properties.difficulty',
		defaultMessage: 'Difficulty',
	},
	maxPlayers: {
		id: 'craftpanel.vendor.properties.max-players',
		defaultMessage: 'Max players',
	},
	motdPlaceholder: {
		id: 'craftpanel.vendor.properties.motd-placeholder',
		defaultMessage: 'A Minecraft Server',
	},
	allowFlight: {
		id: 'craftpanel.vendor.properties.allow-flight',
		defaultMessage: 'Allow flight',
	},
	allowCheats: {
		id: 'craftpanel.vendor.properties.allow-cheats',
		defaultMessage: 'Allow cheats',
	},
	enableWhitelist: {
		id: 'craftpanel.vendor.properties.enable-whitelist',
		defaultMessage: 'Enable whitelist',
	},
	enableSpawnProtection: {
		id: 'craftpanel.vendor.properties.enable-spawn-protection',
		defaultMessage: 'Enable spawn protection',
	},
	protectionRadius: {
		id: 'craftpanel.vendor.properties.protection-radius',
		defaultMessage: 'Protection radius',
	},
	advancedProperties: {
		id: 'craftpanel.vendor.properties.advanced-properties',
		defaultMessage: 'Advanced properties',
	},
	typeHere: {
		id: 'craftpanel.vendor.properties.type-here',
		defaultMessage: 'Type here...',
	},
	otherPropertiesBeforeFilesTab: {
		id: 'craftpanel.vendor.properties.other-properties-before-files-tab',
		defaultMessage: 'All other properties can be edited in server.properties via the',
	},
	noPropertiesFound: {
		id: 'craftpanel.vendor.properties.no-properties-found',
		defaultMessage: 'No properties found',
	},
	noPropertiesMatch: {
		id: 'craftpanel.vendor.properties.no-properties-match',
		defaultMessage: 'No properties match "{query}".',
	},
	propertiesUpdatedTitle: {
		id: 'craftpanel.vendor.properties.updated-title',
		defaultMessage: 'Server properties updated',
	},
	propertiesUpdatedText: {
		id: 'craftpanel.vendor.properties.updated-text',
		defaultMessage: 'Your server properties were successfully changed.',
	},
	propertiesUpdateFailedTitle: {
		id: 'craftpanel.vendor.properties.update-failed-title',
		defaultMessage: 'Failed to update server properties',
	},
	errorOccurred: {
		id: 'craftpanel.vendor.properties.error-occurred',
		defaultMessage: 'An error occurred.',
	},
	groupPerformance: {
		id: 'craftpanel.vendor.properties.group-performance',
		defaultMessage: 'Performance',
	},
	groupResourcePack: {
		id: 'craftpanel.vendor.properties.group-resource-pack',
		defaultMessage: 'Resource Pack',
	},
	optionSurvival: {
		id: 'craftpanel.vendor.properties.option-survival',
		defaultMessage: 'Survival',
	},
	optionCreative: {
		id: 'craftpanel.vendor.properties.option-creative',
		defaultMessage: 'Creative',
	},
	optionHardcore: {
		id: 'craftpanel.vendor.properties.option-hardcore',
		defaultMessage: 'Hardcore',
	},
	optionPeaceful: {
		id: 'craftpanel.vendor.properties.option-peaceful',
		defaultMessage: 'Peaceful',
	},
	optionEasy: {
		id: 'craftpanel.vendor.properties.option-easy',
		defaultMessage: 'Easy',
	},
	optionNormal: {
		id: 'craftpanel.vendor.properties.option-normal',
		defaultMessage: 'Normal',
	},
	optionHard: {
		id: 'craftpanel.vendor.properties.option-hard',
		defaultMessage: 'Hard',
	},
	propertyViewDistance: {
		id: 'craftpanel.vendor.properties.property-view-distance',
		defaultMessage: 'View Distance',
	},
	propertySimulationDistance: {
		id: 'craftpanel.vendor.properties.property-simulation-distance',
		defaultMessage: 'Simulation Distance',
	},
	propertySyncChunkWrites: {
		id: 'craftpanel.vendor.properties.property-sync-chunk-writes',
		defaultMessage: 'Sync Chunk Writes',
	},
	propertyMaxTickTime: {
		id: 'craftpanel.vendor.properties.property-max-tick-time',
		defaultMessage: 'Max Tick Time',
	},
	propertyPlayerIdleTimeout: {
		id: 'craftpanel.vendor.properties.property-player-idle-timeout',
		defaultMessage: 'Player Idle Timeout',
	},
	propertyPauseWhenEmptySeconds: {
		id: 'craftpanel.vendor.properties.property-pause-when-empty-seconds',
		defaultMessage: 'Pause When Empty Seconds',
	},
	propertyResourcePack: {
		id: 'craftpanel.vendor.properties.property-resource-pack',
		defaultMessage: 'Resource Pack',
	},
	propertyResourcePackId: {
		id: 'craftpanel.vendor.properties.property-resource-pack-id',
		defaultMessage: 'Resource Pack Id',
	},
	propertyResourcePackSha1: {
		id: 'craftpanel.vendor.properties.property-resource-pack-sha1',
		defaultMessage: 'Resource Pack Sha1',
	},
	propertyRequireResourcePack: {
		id: 'craftpanel.vendor.properties.property-require-resource-pack',
		defaultMessage: 'Require Resource Pack',
	},
})
const client = injectModrinthClient()
const { serverId, worldId, powerState, busyReasons } = injectModrinthServerContext()
const queryClient = useQueryClient()
const { canUseAdvancedSettings, canUsePowerActions, permissionDeniedMessage } =
	useServerPermissions()
const advancedActionTooltip = computed(() =>
	canUseAdvancedSettings.value ? undefined : permissionDeniedMessage.value,
)
const filesTabLink = computed(
	() => `/hosting/manage/${encodeURIComponent(serverId)}/files?path=/&editing=server.properties`,
)
const serverSettings = injectServerSettings(null)

const searchInput = ref('')

function onFilesTabLinkClick() {
	serverSettings?.closeModal?.()
}

type PropertyDef = { type: 'toggle' } | { type: 'number' } | { type: 'text' }

const KNOWN_PROPERTIES: Record<string, PropertyDef> = {
	allow_cheats: { type: 'toggle' },
	allow_flight: { type: 'toggle' },
	difficulty: { type: 'text' },
	enforce_whitelist: { type: 'toggle' },
	force_gamemode: { type: 'toggle' },
	gamemode: { type: 'text' },
	generate_structures: { type: 'toggle' },
	generator_settings: { type: 'text' },
	hardcore: { type: 'toggle' },
	level_seed: { type: 'text' },
	level_type: { type: 'text' },
	max_players: { type: 'number' },
	max_tick_time: { type: 'number' },
	motd: { type: 'text' },
	pause_when_empty_seconds: { type: 'number' },
	player_idle_timeout: { type: 'number' },
	require_resource_pack: { type: 'toggle' },
	resource_pack: { type: 'text' },
	resource_pack_id: { type: 'text' },
	resource_pack_sha1: { type: 'text' },
	simulation_distance: { type: 'number' },
	spawn_protection: { type: 'number' },
	sync_chunk_writes: { type: 'toggle' },
	view_distance: { type: 'number' },
	white_list: { type: 'toggle' },
}

function getPropertyDef(key: string): PropertyDef {
	return KNOWN_PROPERTIES[key] ?? { type: 'text' }
}

const ADVANCED_GROUPS = [
	{
		label: messages.groupPerformance,
		keys: [
			'view_distance',
			'simulation_distance',
			'sync_chunk_writes',
			'max_tick_time',
			'player_idle_timeout',
			'pause_when_empty_seconds',
		],
	},
	{
		label: messages.groupResourcePack,
		keys: ['resource_pack', 'resource_pack_id', 'resource_pack_sha1', 'require_resource_pack'],
	},
]

type CombinedGamemode = 'survival' | 'creative' | 'hardcore'
const gamemodeItems: CombinedGamemode[] = ['survival', 'creative', 'hardcore']
const difficultyItems = ['peaceful', 'easy', 'normal', 'hard']

function capitalize(str: string): string {
	return str.charAt(0).toUpperCase() + str.slice(1)
}

const optionLabelMessages: Record<string, MessageDescriptor> = {
	survival: messages.optionSurvival,
	creative: messages.optionCreative,
	hardcore: messages.optionHardcore,
	peaceful: messages.optionPeaceful,
	easy: messages.optionEasy,
	normal: messages.optionNormal,
	hard: messages.optionHard,
}

function formatOptionLabel(str: string): string {
	const message = optionLabelMessages[str]
	return message ? formatMessage(message) : capitalize(str)
}

const queryKey = computed(() => ['servers', 'properties', 'v1', serverId, worldId.value])

const { data: propsData } = useQuery({
	queryKey,
	queryFn: () => client.archon.properties_v1.getProperties(serverId, worldId.value!),
	enabled: computed(() => worldId.value !== null),
})

function flattenProperties(data: Archon.Content.v1.PropertiesFields): Record<string, string> {
	const result: Record<string, string> = {}
	if (data.known) {
		for (const [key, value] of Object.entries(data.known)) {
			if (value != null) result[key] = value
		}
	}
	if (data.custom) {
		for (const [key, value] of Object.entries(data.custom)) {
			if (value != null) result[key] = value
		}
	}
	return result
}

const liveProperties = ref<Record<string, string>>({})
const originalProperties = ref<Record<string, string>>({})
const motdValidation = computed(() => validateProfanity(liveProperties.value.motd ?? ''))
const motdBlocksSave = computed(() => !motdValidation.value.valid && !isStaff(auth.user.value))
const detectedMotdProfanity = computed(() =>
	[...new Set(motdValidation.value.matches.map((match) => match.rawText))].join(', '),
)
const motdValidationMessage = computed(() =>
	motdValidation.value.valid
		? undefined
		: formatMessage(messages.motdProfanity, { value: detectedMotdProfanity.value }),
)
let previousSpawnProtection = '16'

function syncFormFromData() {
	if (!propsData.value) return
	const flat = flattenProperties(propsData.value)
	liveProperties.value = { ...flat }
	originalProperties.value = { ...flat }
	const sp = flat.spawn_protection
	if (sp && sp !== '0') {
		previousSpawnProtection = sp
	}
}

const hasNoProperties = computed(() => Object.keys(liveProperties.value).length === 0)

const hasUnsavedChanges = computed(() =>
	Object.keys(liveProperties.value).some(
		(key) => liveProperties.value[key] !== originalProperties.value[key],
	),
)

watch(
	propsData,
	(newData) => {
		if (newData && !hasUnsavedChanges.value) {
			syncFormFromData()
		}
	},
	{ immediate: true },
)

watch(powerState, () => {
	queryClient.invalidateQueries({ queryKey: queryKey.value })
})

const combinedGamemode = computed<CombinedGamemode>({
	get() {
		if (liveProperties.value.hardcore === 'true') return 'hardcore'
		if (liveProperties.value.gamemode === 'creative') return 'creative'
		return 'survival'
	},
	set(value) {
		if (value === 'hardcore') {
			liveProperties.value.gamemode = 'survival'
			liveProperties.value.hardcore = 'true'
			liveProperties.value.difficulty = 'hard'
		} else {
			liveProperties.value.gamemode = value
			liveProperties.value.hardcore = 'false'
		}
	},
})

const selectedDifficulty = computed({
	get: () => liveProperties.value.difficulty ?? 'normal',
	set: (v: string) => {
		liveProperties.value.difficulty = v
	},
})

const whitelistEnabled = computed({
	get: () => liveProperties.value.white_list === 'true',
	set: (v: boolean) => {
		liveProperties.value.white_list = v ? 'true' : 'false'
		liveProperties.value.enforce_whitelist = v ? 'true' : 'false'
	},
})

const spawnProtectionEnabled = computed({
	get: () => {
		const val = liveProperties.value.spawn_protection
		return val !== undefined && val !== '0'
	},
	set: (enabled: boolean) => {
		if (enabled) {
			liveProperties.value.spawn_protection = previousSpawnProtection || '16'
		} else {
			previousSpawnProtection = liveProperties.value.spawn_protection || '16'
			liveProperties.value.spawn_protection = '0'
		}
	},
})

function buildPatch(): Archon.Content.v1.PatchPropertiesFields {
	const known: Record<string, string> = {}
	const custom: Record<string, string> = {}

	for (const key of Object.keys(liveProperties.value)) {
		if (liveProperties.value[key] === originalProperties.value[key]) continue
		if (key in KNOWN_PROPERTIES) {
			known[key] = liveProperties.value[key]
		} else {
			custom[key] = liveProperties.value[key]
		}
	}

	const patch: Archon.Content.v1.PatchPropertiesFields = {}
	if (Object.keys(known).length > 0) {
		patch.known = known as Archon.Content.v1.KnownPropertiesFields
	}
	if (Object.keys(custom).length > 0) {
		patch.custom = custom
	}
	return patch
}

const { mutateAsync: savePropertiesMutation, isPending: isUpdating } = useMutation({
	mutationFn: () =>
		client.archon.properties_v1.patchProperties(serverId, worldId.value!, buildPatch()),
	onSuccess: async () => {
		await queryClient.invalidateQueries({ queryKey: queryKey.value })
		syncFormFromData()
		addNotification({
			type: 'success',
			title: formatMessage(messages.propertiesUpdatedTitle),
			text: formatMessage(messages.propertiesUpdatedText),
		})
	},
	onError: (error) => {
		addNotification({
			type: 'error',
			title: formatMessage(messages.propertiesUpdateFailedTitle),
			text: error instanceof Error ? error.message : formatMessage(messages.errorOccurred),
		})
	},
})

async function saveProperties() {
	if (!canUseAdvancedSettings.value) return
	if (motdBlocksSave.value) {
		addNotification({
			type: 'error',
			title: formatMessage(commonMessages.errorNotificationTitle),
			text: formatMessage(messages.motdProfanitySaveError, {
				value: detectedMotdProfanity.value,
			}),
		})
		return false
	}
	await savePropertiesMutation()
}

function resetProperties() {
	syncFormFromData()
}

const advancedGroupedProperties = computed(() =>
	ADVANCED_GROUPS.map((group) => ({
		label: formatMessage(group.label),
		properties: group.keys.filter((key) => key in liveProperties.value),
	})).filter((g) => g.properties.length > 0),
)

const fuse = computed(() => {
	const entries = Object.entries(liveProperties.value).map(([key, value]) => ({
		key,
		value: String(value),
	}))
	return new Fuse(entries, { keys: ['key', 'value'], threshold: 0.2 })
})

const filteredProperties = computed(() => {
	if (!searchInput.value?.trim()) return liveProperties.value
	const results = fuse.value.search(searchInput.value)
	return Object.fromEntries(results.map(({ item }) => [item.key, liveProperties.value[item.key]]))
})

const isSearchActive = computed(() => !!searchInput.value?.trim())
const hasNoResults = computed(
	() => isSearchActive.value && Object.keys(filteredProperties.value).length === 0,
)

function isPropertyVisible(key: string): boolean {
	if (!isSearchActive.value) return true
	return key in filteredProperties.value
}

function hasVisibleProperties(group: { properties: string[] }): boolean {
	return group.properties.some((key) => isPropertyVisible(key))
}

const hasVisibleAdvancedProperties = computed(() =>
	advancedGroupedProperties.value.some((group) => hasVisibleProperties(group)),
)

const propertyNameMessages: Record<string, MessageDescriptor> = {
	view_distance: messages.propertyViewDistance,
	simulation_distance: messages.propertySimulationDistance,
	sync_chunk_writes: messages.propertySyncChunkWrites,
	max_tick_time: messages.propertyMaxTickTime,
	player_idle_timeout: messages.propertyPlayerIdleTimeout,
	pause_when_empty_seconds: messages.propertyPauseWhenEmptySeconds,
	resource_pack: messages.propertyResourcePack,
	resource_pack_id: messages.propertyResourcePackId,
	resource_pack_sha1: messages.propertyResourcePackSha1,
	require_resource_pack: messages.propertyRequireResourcePack,
}

function formatPropertyName(name: string): string {
	const message = propertyNameMessages[name]
	if (message) return formatMessage(message)
	return name
		.split('_')
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
		.join(' ')
}
</script>
