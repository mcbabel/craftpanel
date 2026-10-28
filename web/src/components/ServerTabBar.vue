<template>
	<nav
		class="fixed inset-x-0 bottom-0 z-20 border-0 border-t border-solid border-surface-4 bg-surface-2 pb-[env(safe-area-inset-bottom)] md:hidden"
		:aria-label="label"
	>
		<ul
			class="m-0 grid list-none p-0"
			:style="{ gridTemplateColumns: `repeat(${visible.length}, minmax(0, 1fr))` }"
		>
			<li v-for="link in visible" :key="link.href" class="min-w-0">
				<RouterLink
					:to="link.href"
					replace
					class="flex h-14 flex-col items-center justify-center gap-0.5 px-0.5 text-[11px] font-semibold"
					:class="isActive(link.href) ? 'text-brand' : 'text-secondary'"
					:aria-current="isActive(link.href) ? 'page' : undefined"
					:aria-label="link.label"
				>
					<component :is="link.icon" class="size-5 shrink-0" aria-hidden="true" />
					<span class="max-w-full truncate">{{ link.shortLabel ?? link.label }}</span>
				</RouterLink>
			</li>
		</ul>
	</nav>
</template>

<script setup lang="ts">
import { type Component, computed } from 'vue'
import { RouterLink, useRoute } from 'vue-router'

const props = defineProps<{
	links: { label: string; shortLabel?: string; href: string; icon: Component; shown?: boolean }[]
	label: string
}>()

const route = useRoute()

const visible = computed(() => props.links.filter((link) => link.shown ?? true))
const home = computed(() => props.links[0]?.href)

function isActive(href: string): boolean {
	if (route.path === href) return true
	return href !== home.value && route.path.startsWith(`${href}/`)
}
</script>
