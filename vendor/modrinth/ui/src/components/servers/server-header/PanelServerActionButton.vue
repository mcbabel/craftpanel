<template>
	<div class="contents">
		<div class="flex flex-row items-center gap-2 rounded-lg">
			<Button
				v-if="isInstalling"
				type="colored"
				color="brand"
				size="xl"
				disabled
				class="flex-shrink-0"
			>
				<LoaderCircleIcon class="size-5 animate-spin" />
				{{ formatMessage(commonMessages.installingLabel) }}
			</Button>

			<template v-else-if="showRestartButton">
				<Button
					v-tooltip="busyTooltip"
					type="colored"
					color="orange"
					size="xl"
					:disabled="!canTakeAction"
					@click="handlePrimaryAction"
				>
					<UpdatedIcon />
					<span>{{ primaryActionText }}</span>
				</Button>

				<SplitButton
					v-tooltip="busyTooltip"
					type="colored"
					color="red"
					size="xl"
					:options="stopMenuOptions"
					:menu-label="formatMessage(messages.forceStopOptions)"
					:primary-disabled="!canTakeAction"
					:menu-disabled="!canKill"
					@click="initiateAction('Stop')"
				>
					<StopCircleIcon />
					{{
						isStopping
							? formatMessage(messages.stopping)
							: formatMessage(commonMessages.stopButton)
					}}
				</SplitButton>
			</template>

			<template v-else-if="isStopping">
				<SplitButton
					v-tooltip="busyTooltip"
					type="colored"
					color="red"
					size="xl"
					:options="stopMenuOptions"
					:menu-label="formatMessage(messages.forceStopOptions)"
					:primary-disabled="true"
					:menu-disabled="!canKill"
					class="opacity-60"
				>
					<StopCircleIcon />
					{{ formatMessage(messages.stopping) }}
				</SplitButton>
			</template>

			<template v-else>
				<Button
					v-tooltip="busyTooltip"
					type="colored"
					color="brand"
					size="xl"
					:disabled="!canTakeAction"
					@click="handlePrimaryAction"
				>
					<PlayIcon />
					<span>{{ primaryActionText }}</span>
				</Button>
			</template>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	LoaderCircleIcon,
	PlayIcon,
	SlashIcon,
	StopCircleIcon,
	UpdatedIcon,
} from '@modrinth/assets'
import { computed } from 'vue'

import type { ButtonMenuOption } from '#ui/components/base/buttons'
import { Button, SplitButton } from '#ui/components/base/buttons'
import { defineMessages, useVIntl } from '#ui/composables/i18n'
import { commonMessages } from '#ui/utils/common-messages'

import { useServerPowerAction } from './use-server-power-action'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	forceStopOptions: {
		id: 'craftpanel.vendor.power.force-stop-options',
		defaultMessage: 'Force stop options',
	},
	stopping: {
		id: 'craftpanel.vendor.power.stopping',
		defaultMessage: 'Stopping',
	},
	killServer: {
		id: 'craftpanel.vendor.power.kill-server',
		defaultMessage: 'Kill server',
	},
})

const props = withDefaults(
	defineProps<{
		disabled?: boolean
	}>(),
	{
		disabled: false,
	},
)

const {
	isInstalling,
	isStopping,
	showRestartButton,
	busyTooltip,
	canTakeAction,
	canKill,
	primaryActionText,
	initiateAction,
	handlePrimaryAction,
} = useServerPowerAction({
	disabled: computed(() => props.disabled),
})

const stopMenuOptions = computed<ButtonMenuOption[]>(() => [
	{
		id: 'kill_server',
		label: formatMessage(messages.killServer),
		icon: SlashIcon,
		action: () => initiateAction('Kill'),
	},
])
</script>
