const PROFILE_PATH = /^\/(user|organization)\//

function unlinked(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(unlinked)
	if (typeof value !== 'object' || value === null || !('to' in value)) return value
	if (typeof value.to !== 'string' || !PROFILE_PATH.test(value.to)) return value
	const { to: _, ...rest } = value
	return rest
}

export function withoutProfileLinks<T extends { props: Record<string, unknown> }>(event: T): T {
	const props = Object.fromEntries(
		Object.entries(event.props).map(([key, value]) => [key, unlinked(value)]),
	)
	return { ...event, props }
}
