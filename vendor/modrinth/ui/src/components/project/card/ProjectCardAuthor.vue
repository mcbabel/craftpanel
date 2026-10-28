<script setup lang="ts">
import { ExternalIcon } from '@modrinth/assets'

import { defineMessages, useVIntl } from '../../../composables/i18n'
import { AutoLink } from '../../base'

export type ProjectCardAuthorDetails = {
	name: string
	link?: string
}

const { formatMessage } = useVIntl()

const messages = defineMessages({
	by: {
		id: 'craftpanel.vendor.project.by-author',
		defaultMessage: 'by',
	},
})

defineProps<{
	author: ProjectCardAuthorDetails
}>()
</script>

<template>
	<!-- author will try to be full width, but no longer than the proper fit. but this also allows it to truncate if necessary -->
	<!-- the weird padding and negative margin are to include the potential hover underline in bounding box which affects rendering on firefox -->
	<span
		class="line-clamp-1 break-all text-secondary font-normal max-w-fit w-full pb-[2px] mb-[-2px]"
	>
		{{ formatMessage(messages.by) }}
		<AutoLink
			:to="author.link"
			:class="
				author.link
					? 'custom-focus-indicator text-inherit outline-none group focus-visible:text-[--color-focus-ring] smart-clickable:allow-pointer-events'
					: ''
			"
		>
			<span
				class="group-focus:underline group-focus:brightness-[--hover-brightness] group-hover:brightness-[--hover-brightness] group-hover:underline"
			>
				{{ author.name }}
			</span>
			<ExternalIcon v-if="author.link?.startsWith('http')" class="shrink-0 ml-1" />
		</AutoLink>
	</span>
</template>
