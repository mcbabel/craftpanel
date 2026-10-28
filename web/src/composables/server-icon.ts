import { useQuery } from '@tanstack/vue-query'
import { computed, type ComputedRef, type MaybeRefOrGetter, toValue } from 'vue'

import { api, isApiRequestError, type Ulid } from '@/api'

const STALE_MS = 60_000

// The vendored icon editor updates every query below ['servers', 'detail', id, 'icon'] after a
// change, so the header and the server list pick up a new icon without asking again.
export function serverIconKey(serverId: Ulid) {
	return ['servers', 'detail', serverId, 'icon', 'panel'] as const
}

function asDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader()
		reader.onload = () => resolve(String(reader.result))
		reader.onerror = () => reject(reader.error)
		reader.readAsDataURL(blob)
	})
}

export async function fetchServerIcon(serverId: Ulid): Promise<string | null> {
	try {
		return await asDataUrl(await api.servers.icon(serverId))
	} catch (cause) {
		if (!isApiRequestError(cause) || cause.status !== 404) {
			console.debug('Server icon fetch failed:', cause)
		}
		return null
	}
}

export function useServerIcon(serverId: MaybeRefOrGetter<Ulid>): ComputedRef<string | undefined> {
	const { data } = useQuery({
		queryKey: computed(() => serverIconKey(toValue(serverId))),
		queryFn: () => fetchServerIcon(toValue(serverId)),
		staleTime: STALE_MS,
	})
	return computed(() => data.value ?? undefined)
}
