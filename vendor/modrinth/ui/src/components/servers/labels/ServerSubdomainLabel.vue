<template>
	<div
		v-if="subdomain && !isHidden"
		v-tooltip="formatMessage(messages.copyCustomUrl)"
		class="flex min-w-0 flex-row items-center gap-2 truncate hover:cursor-pointer hover:underline"
		data-subdomain-label
		@click.stop.prevent
	>
		<Separator v-if="!noSeparator" />

		<div class="flex flex-row items-center gap-1.5">
			<LinkIcon />
			<div
				class="flex min-w-0 font-medium text-sm text-nowrap"
				:class="props.subdomain ? 'hover:underline' : ''"
				@click="copySubdomain"
			>
				{{ subdomain }}.modrinth.gg
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import { LinkIcon } from '@modrinth/assets'
import { injectNotificationManager } from '@modrinth/ui'
import { useStorage } from '@vueuse/core'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { defineMessages, useVIntl } from '#ui/composables/i18n'

import Separator from './Separator.vue'

const { addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	copyCustomUrl: {
		id: 'craftpanel.vendor.subdomain.copy-custom-url',
		defaultMessage: 'Copy custom URL',
	},
	customUrlCopied: {
		id: 'craftpanel.vendor.subdomain.custom-url-copied',
		defaultMessage: 'Custom URL copied',
	},
	customUrlCopiedText: {
		id: 'craftpanel.vendor.subdomain.custom-url-copied-text',
		defaultMessage: "Your server's URL has been copied to your clipboard.",
	},
})

const props = defineProps<{
	subdomain: string
	serverId?: string
	noSeparator?: boolean
}>()

const copySubdomain = () => {
	navigator.clipboard.writeText(props.subdomain + '.modrinth.gg')
	addNotification({
		title: formatMessage(messages.customUrlCopied),
		text: formatMessage(messages.customUrlCopiedText),
		type: 'success',
	})
}

const route = useRoute()
const serverId = props.serverId || (route.params.id as string)

const userPreferences = useStorage(`pyro-server-${serverId}-preferences`, {
	hideSubdomainLabel: false,
})

const isHidden = computed(() => userPreferences.value.hideSubdomainLabel)
</script>
