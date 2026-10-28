<template>
	<div class="flex flex-col gap-6 lg:flex-row lg:items-start">
		<nav
			:aria-label="formatMessage(messages.navigation)"
			class="flex shrink-0 flex-col gap-4 lg:sticky lg:top-6 lg:w-60"
		>
			<h1 class="m-0 text-2xl font-extrabold text-contrast">
				{{ formatMessage(messages.title) }}
			</h1>

			<div class="flex gap-1.5 overflow-x-auto pb-1 lg:hidden">
				<RouterLink
					v-for="page in pages"
					:key="page.name"
					:to="{ name: page.name }"
					class="flex shrink-0 items-center gap-2 rounded-full border border-solid px-3 py-1.5 text-sm font-semibold no-underline"
					:class="
						page.name === route.name
							? 'border-brand bg-brand-highlight text-brand'
							: 'border-surface-5 text-primary hover:bg-surface-3'
					"
				>
					<component :is="page.icon" v-if="page.icon" aria-hidden="true" class="size-4" />
					{{ formatMessage(page.label!) }}
				</RouterLink>
			</div>

			<div class="hidden flex-col gap-4 lg:flex">
				<div v-for="group in sections" :key="group.id" class="flex flex-col gap-1">
					<span class="px-3 text-sm font-semibold text-secondary">
						{{ formatMessage(adminGroups[group.id]) }}
					</span>
					<RouterLink
						v-for="page in group.pages"
						:key="page.name"
						:to="{ name: page.name }"
						class="flex items-center gap-2.5 rounded-xl px-3 py-2 font-semibold no-underline transition-colors"
						:class="
							page.name === route.name
								? 'bg-brand-highlight text-brand'
								: 'text-primary hover:bg-surface-3'
						"
						:aria-current="page.name === route.name ? 'page' : undefined"
					>
						<component :is="page.icon" v-if="page.icon" aria-hidden="true" class="size-5" />
						{{ formatMessage(page.label!) }}
					</RouterLink>
				</div>
			</div>
		</nav>

		<div class="min-w-0 flex-1">
			<RouterView />
		</div>
	</div>
</template>

<script setup lang="ts">
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'

import { type AdminGroup, adminGroups, adminPages } from '@/pages/admin/routes'

const { formatMessage } = useVIntl()
const route = useRoute()

const messages = defineMessages({
	title: { id: 'panel.admin.title', defaultMessage: 'Administration' },
	navigation: { id: 'panel.admin.navigation', defaultMessage: 'Administration pages' },
})

const pages = adminPages.filter((page) => page.label !== null)

const sections = computed(() =>
	(Object.keys(adminGroups) as AdminGroup[])
		.map((id) => ({ id, pages: pages.filter((page) => page.group === id) }))
		.filter((group) => group.pages.length > 0),
)
</script>
