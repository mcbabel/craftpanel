import { computed, ref } from 'vue'

import { api, type Invitation, isApiRequestError, type ServerMember, type Ulid } from '@/api'

const invitations = ref<Invitation[]>([])
const busy = ref<Record<Ulid, 'accept' | 'decline'>>({})
let inflight: Promise<void> | null = null

const count = computed(() => invitations.value.length)

async function load(): Promise<void> {
	try {
		const list = await api.access.invitations()
		invitations.value = list.invitations
	} catch {
		return
	} finally {
		inflight = null
	}
}

function refresh(): Promise<void> {
	inflight ??= load()
	return inflight
}

function settle(id: Ulid): void {
	invitations.value = invitations.value.filter((invitation) => invitation.id !== id)
}

async function act<T>(id: Ulid, kind: 'accept' | 'decline', run: () => Promise<T>): Promise<T> {
	busy.value = { ...busy.value, [id]: kind }
	try {
		const result = await run()
		settle(id)
		return result
	} catch (cause) {
		if (isApiRequestError(cause) && (cause.status === 404 || cause.status === 409)) settle(id)
		throw cause
	} finally {
		const { [id]: _, ...rest } = busy.value
		busy.value = rest
	}
}

function accept(id: Ulid): Promise<ServerMember> {
	return act(id, 'accept', () => api.access.acceptInvitation(id))
}

function decline(id: Ulid): Promise<void> {
	return act(id, 'decline', () => api.access.declineInvitation(id))
}

function forget(): void {
	invitations.value = []
	busy.value = {}
}

export function useInvitations() {
	return { invitations, count, busy, refresh, accept, decline, forget }
}
