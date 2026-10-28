import { describe, expect, it, vi } from 'vitest'

import type { Operation, Server } from '@/api'

vi.mock('@modrinth/ui/src/composables/i18n.ts', () => ({
	defineMessages: (descriptors: Record<string, unknown>) => descriptors,
}))
vi.mock('@modrinth/ui/src/composables/server-installation-tracker.ts', () => ({
	useServerInstallationTracker: () => ({}),
}))
vi.mock('@modrinth/ui/src/providers/server-context.ts', () => ({
	provideModrinthServerContext: () => undefined,
}))

const { installationKeyOf } = await import('./server-context')

const server = {
	loader: 'fabric',
	loader_version: '0.16.9',
	game_version: '1.21.1',
	upstream: { kind: 'modpack', project_id: 'gHYUOczU', version_id: 'v6' },
} as Server

function operation(over: Partial<Operation>): Operation {
	return { kind: 'install_loader', current_file: null, ...over } as Operation
}

describe('What the install banner says is being installed', () => {
	it('names the mod or plugin a content run is on, without its folder', () => {
		expect(
			installationKeyOf(
				operation({ kind: 'install_content', current_file: 'mods/sodium-fabric-0.6.0.jar' }),
				server,
			),
		).toEqual({ type: 'local_modpack', filename: 'sodium-fabric-0.6.0.jar' })
		expect(
			installationKeyOf(operation({ kind: 'update_content', current_file: 'lithium.jar' }), server),
		).toEqual({ type: 'local_modpack', filename: 'lithium.jar' })
	})

	it('says nothing particular before a content run has reached its first file', () => {
		expect(installationKeyOf(operation({ kind: 'install_content' }), server)).toEqual({
			type: 'unknown',
		})
	})

	it('calls a modpack run a modpack, and a loader run the platform', () => {
		expect(installationKeyOf(operation({ kind: 'install_modpack' }), server)).toMatchObject({
			type: 'modrinth_modpack',
			project_id: 'gHYUOczU',
		})
		expect(installationKeyOf(operation({ kind: 'install_loader' }), server)).toEqual({
			type: 'platform',
			platform: 'fabric',
			platform_version: '0.16.9',
			game_version: '1.21.1',
		})
	})
})
