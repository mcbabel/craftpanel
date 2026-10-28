import { describe, expect, it } from 'vitest'

import type { GameVersionEntry, LoaderBuild, LoaderId, LoaderInfo } from '@/api'

import {
	type CatalogClient,
	modpackFilter,
	modpackFromState,
	modpackState,
	paperBuild,
	runnableVersions,
	toVersionTags,
	useCreationCatalog,
} from './creation-catalog'

function loader(id: LoaderId, kind: LoaderInfo['kind'], wave: 1 | 2 = 1): LoaderInfo {
	return {
		id,
		name: id,
		kind,
		install_kind: wave === 1 ? 'download' : 'installer',
		has_loader_versions: id !== 'vanilla',
		supports_properties: kind !== 'proxy',
		supports_content: true,
		source: 'mojang',
		wave,
	}
}

function build(id: string, stable = true, channel: LoaderBuild['channel_tag'] = null): LoaderBuild {
	return { id, label: id, stable, channel_tag: channel, released: null }
}

const release = (version: string): GameVersionEntry => ({ version, version_type: 'release' })
const snapshot = (version: string): GameVersionEntry => ({ version, version_type: 'snapshot' })

const VERSIONS: Partial<Record<LoaderId, GameVersionEntry[]>> = {
	vanilla: [snapshot('25w01a'), release('1.21.1'), release('1.20.1')],
	paper: [release('1.21.1')],
	fabric: [snapshot('25w01a'), release('1.21.1')],
	leaf: [release('1.21.1'), release('1.20.1')],
	velocity: [release('3.4.0')],
}

const BUILDS: Record<string, LoaderBuild[]> = {
	'paper 1.21.1': [build('133'), build('134', false, 'BETA')],
	'fabric 25w01a': [build('0.16.9'), build('0.16.10', false)],
	'leaf 1.21.1': [build('40')],
	'leaf 1.20.1': [],
}

function fakeClient(asked: string[] = []): CatalogClient {
	return {
		loaders: async () => ({
			loaders: [
				loader('vanilla', 'vanilla'),
				loader('paper', 'server'),
				loader('leaf', 'server'),
				loader('fabric', 'modloader'),
				loader('velocity', 'proxy'),
				loader('forge', 'modloader', 2),
			],
		}),
		gameVersions: async (id) => ({
			loader: id,
			game_versions: VERSIONS[id] ?? [],
			cached_until: '',
		}),
		builds: async (id, gameVersion) => {
			asked.push(`${id} ${gameVersion}`)
			return {
				loader: id,
				game_version: gameVersion,
				builds: BUILDS[`${id} ${gameVersion}`] ?? [],
				truncated: false,
				cached_until: '',
			}
		},
	} as CatalogClient
}

describe('the catalogue behind the setup flow', () => {
	it('offers only what the panel can install, and vanilla only as its own choice', async () => {
		const catalog = useCreationCatalog(fakeClient())
		await catalog.load()

		expect(catalog.ready.value).toBe(true)
		expect(catalog.flowLoaders.value).toEqual(['paper', 'leaf', 'fabric', 'velocity'])
		expect(catalog.modpackLoaders.value).toEqual(['fabric'])
	})

	it('lists every game version once, the vanilla ones first', async () => {
		const catalog = useCreationCatalog(fakeClient())
		await catalog.load()

		expect(catalog.gameVersionTags.value.map((tag) => tag.version)).toEqual([
			'25w01a',
			'1.21.1',
			'1.20.1',
			'3.4.0',
		])
		expect(catalog.gameVersionTags.value[0].version_type).toBe('snapshot')
	})

	it('describes Fabric with one list of loader versions for every game version', async () => {
		const asked: string[] = []
		const catalog = useCreationCatalog(fakeClient(asked))
		await catalog.load()

		const manifest = await catalog.manifest('fabric')

		expect(asked).toEqual(['fabric 25w01a'])
		expect(manifest.gameVersions[0]).toEqual({
			id: '${modrinth.gameVersion}',
			stable: true,
			loaders: [
				{ id: '0.16.9', url: '', stable: true },
				{ id: '0.16.10', url: '', stable: false },
			],
		})
		expect(manifest.gameVersions.slice(1).map((entry) => entry.id)).toEqual(['25w01a', '1.21.1'])
	})

	it('describes a loader with builds per game version one game version at a time', async () => {
		const catalog = useCreationCatalog(fakeClient())
		await catalog.load()

		const manifest = await catalog.manifest('leaf')

		expect(manifest.gameVersions).toEqual([
			{ id: '1.21.1', stable: true, loaders: [{ id: '40', url: '', stable: true }] },
			{ id: '1.20.1', stable: true, loaders: [] },
		])
	})

	it('answers the Paper questions of the flow from the panel', async () => {
		const asked: string[] = []
		const catalog = useCreationCatalog(fakeClient(asked))
		await catalog.load()

		const project = await catalog.paper.versions_v3.getProject()
		const builds = await catalog.paper.versions_v3.getBuilds('1.21.1')
		await catalog.paper.versions_v3.getBuilds('1.21.1')

		expect(Object.values(project.versions).flat()).toEqual(['1.21.1'])
		expect(builds.builds).toEqual([
			{ id: 133, time: '', channel: 'STABLE' },
			{ id: 134, time: '', channel: 'BETA' },
		])
		expect(asked, 'a second look is answered from what was fetched').toEqual(['paper 1.21.1'])
	})

	it('calls a Paper build the panel does not count as stable a beta', () => {
		expect(paperBuild(build('7', false)).channel).toBe('BETA')
		expect(paperBuild(build('8', false, 'ALPHA')).channel).toBe('ALPHA')
	})

	it('builds tags from nothing when nothing is known', () => {
		expect(toVersionTags([[], []])).toEqual([])
	})
})

describe('modpacks for a server', () => {
	it('searches only server packs for the loaders the panel can install', () => {
		expect(modpackFilter(['fabric'])).toBe(
			'project_types = "modpack" AND (categories = "fabric") AND ' +
				'(client_side = "optional" OR client_side = "required") AND server_side = "required"',
		)
		expect(modpackFilter(['fabric', 'quilt'])).toContain(
			'(categories = "fabric" OR categories = "quilt")',
		)
	})

	it('picks only versions the panel can run', () => {
		const versions = [
			{ id: 'forge', loaders: ['forge'] },
			{ id: 'both', loaders: ['quilt', 'fabric'] },
			{ id: 'unknown', loaders: [] },
		]
		expect(runnableVersions(versions, ['fabric']).map((version) => version.id)).toEqual([
			'both',
			'unknown',
		])
	})

	it('carries a picked pack through the history state and back', () => {
		const state = modpackState({
			projectId: 'AAA',
			name: 'Sky Pack',
			iconUrl: 'https://cdn.example/icon.png',
		})
		expect(modpackFromState(state)).toEqual({
			projectId: 'AAA',
			versionId: null,
			name: 'Sky Pack',
			iconUrl: 'https://cdn.example/icon.png',
		})
		expect(modpackFromState({ back: '/servers', position: 3 })).toBeNull()
		expect(modpackFromState({ modpack: { projectId: 42 } })).toBeNull()
		expect(modpackFromState(null)).toBeNull()
	})
})
