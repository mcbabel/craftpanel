import type { ServerNet } from './types'

export function serverAddress(net: ServerNet | null | undefined): string | null {
	if (!net) return null
	if (net.domain) return net.domain
	if (net.ip) return `${net.ip}:${net.port}`
	return null
}
