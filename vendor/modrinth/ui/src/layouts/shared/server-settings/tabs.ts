import type { Archon } from '@modrinth/api-client'
import {
	DatabaseBackupIcon,
	ListIcon,
	SettingsIcon,
	ShieldIcon,
	TextQuoteIcon,
	VersionIcon,
	WrenchIcon,
} from '@modrinth/assets'
import type { Component } from 'vue'

export type ServerSettingsTabId =
	| 'general'
	| 'installation'
	| 'network'
	| 'backups'
	| 'properties'
	| 'advanced'
	| 'support'

export interface ServerSettingsTabContext {
	serverId: string
	ownerId: string
	serverStatus?: Archon.Servers.v0.Status | null
	isOwner: boolean
	isAdmin: boolean
}

export interface ServerSettingsTabDefinition {
	id: ServerSettingsTabId
	label: string
	icon: Component
	href?: (ctx: ServerSettingsTabContext) => string
	external?: boolean
	shown?: (ctx: ServerSettingsTabContext) => boolean
}

export const serverSettingsTabDefinitions: ServerSettingsTabDefinition[] = [
	{
		id: 'general',
		label: 'General',
		icon: SettingsIcon,
	},
	{
		id: 'installation',
		label: 'Installation',
		icon: WrenchIcon,
	},
	{
		id: 'network',
		label: 'Network',
		icon: VersionIcon,
	},
	{
		id: 'backups',
		label: 'Backups',
		icon: DatabaseBackupIcon,
	},
	{
		id: 'properties',
		label: 'Properties',
		icon: ListIcon,
		shown: ({ serverStatus }) => serverStatus !== 'installing',
	},
	{
		id: 'advanced',
		label: 'Advanced',
		icon: TextQuoteIcon,
	},
	{
		id: 'support',
		label: 'Support',
		icon: ShieldIcon,
		shown: ({ isAdmin }) => isAdmin,
	},
]
