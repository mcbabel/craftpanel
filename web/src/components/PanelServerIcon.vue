<template>
	<ServerIcon :image="image" :disabled="disabled" />
</template>

<script setup lang="ts">
import { ServerIcon } from '@modrinth/ui'

import type { Ulid } from '@/api'
import { useServerIcon } from '@/composables/server-icon'

const props = defineProps<{ serverId: Ulid; disabled?: boolean }>()

const image = useServerIcon(() => props.serverId)
</script>
