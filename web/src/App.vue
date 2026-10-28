<template>
	<div v-if="ready" class="flex min-h-full flex-col">
		<RouterView class="flex-1" />
		<SourceFooter />
	</div>
	<div v-else class="grid min-h-full place-items-center text-secondary">
		<SpinnerIcon class="size-8 animate-spin" />
	</div>
</template>

<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { ref } from 'vue'
import { RouterView, useRouter } from 'vue-router'

import SourceFooter from '@/components/SourceFooter.vue'

const ready = ref(false)
const settle = () => {
	ready.value = true
}

useRouter().isReady().then(settle, settle)
</script>
