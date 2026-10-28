import {
	CloudIcon,
	CoffeeIcon,
	GaugeIcon,
	GlobeIcon,
	MailIcon,
	SettingsIcon,
	UserPlusIcon,
	UsersIcon,
} from '@modrinth/assets'
import { defineMessages, type MessageDescriptor } from '@modrinth/ui'
import type { Component } from 'vue'
import type { RouteRecordSingleView } from 'vue-router'

export type AdminGroup = 'overview' | 'people' | 'connections' | 'system'

export interface AdminPage {
	name: string
	path: string
	component: RouteRecordSingleView['component']
	label: MessageDescriptor | null
	icon: Component | null
	group: AdminGroup
}

const messages = defineMessages({
	overview: { id: 'panel.menu.overview', defaultMessage: 'Overview' },
	users: { id: 'panel.menu.users', defaultMessage: 'Users' },
	playit: { id: 'panel.menu.playit-all', defaultMessage: 'Public addresses' },
	mail: { id: 'panel.menu.mail', defaultMessage: 'Mail' },
	registrations: { id: 'panel.menu.registrations', defaultMessage: 'Sign-ups' },
	drive: { id: 'panel.menu.drive', defaultMessage: 'Google Drive' },
	runtimes: { id: 'panel.menu.runtimes', defaultMessage: 'Java runtimes' },
	settings: { id: 'panel.menu.panel-settings', defaultMessage: 'Panel settings' },
})

export const adminGroups: Record<AdminGroup, MessageDescriptor> = defineMessages({
	overview: { id: 'panel.admin.group.overview', defaultMessage: 'Machine' },
	people: { id: 'panel.admin.group.people', defaultMessage: 'People' },
	connections: { id: 'panel.admin.group.connections', defaultMessage: 'Connections' },
	system: { id: 'panel.admin.group.system', defaultMessage: 'System' },
})

export const adminPages: AdminPage[] = [
	{
		name: 'admin-overview',
		path: 'admin',
		component: () => import('@/pages/admin/Statistics.vue'),
		label: messages.overview,
		icon: GaugeIcon,
		group: 'overview',
	},
	{
		name: 'admin-users',
		path: 'admin/users',
		component: () => import('@/pages/admin/Users.vue'),
		label: messages.users,
		icon: UsersIcon,
		group: 'people',
	},
	{
		name: 'admin-registrations',
		path: 'admin/registrations',
		component: () => import('@/pages/admin/Registrations.vue'),
		label: messages.registrations,
		icon: UserPlusIcon,
		group: 'people',
	},
	{
		name: 'admin-playit',
		path: 'admin/playit',
		component: () => import('@/pages/admin/Playit.vue'),
		label: messages.playit,
		icon: GlobeIcon,
		group: 'connections',
	},
	{
		name: 'admin-mail',
		path: 'admin/mail',
		component: () => import('@/pages/admin/Mail.vue'),
		label: messages.mail,
		icon: MailIcon,
		group: 'connections',
	},
	{
		name: 'admin-drive',
		path: 'admin/drive',
		component: () => import('@/pages/admin/Drive.vue'),
		label: messages.drive,
		icon: CloudIcon,
		group: 'connections',
	},
	{
		name: 'admin-runtimes',
		path: 'admin/runtimes',
		component: () => import('@/pages/admin/Runtimes.vue'),
		label: messages.runtimes,
		icon: CoffeeIcon,
		group: 'system',
	},
	{
		name: 'admin-settings',
		path: 'admin/settings',
		component: () => import('@/pages/admin/Settings.vue'),
		label: messages.settings,
		icon: SettingsIcon,
		group: 'system',
	},
]
