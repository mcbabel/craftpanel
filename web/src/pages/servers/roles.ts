import { defineMessages, type MessageDescriptor } from '@modrinth/ui'

import type { ServerRole } from '@/api'

export const roleMessages = defineMessages({
	ownerRole: { id: 'craftpanel.access.role-owner', defaultMessage: 'Owner' },
	editorRole: { id: 'craftpanel.access.role-editor', defaultMessage: 'Editor' },
	editorDescription: {
		id: 'craftpanel.access.role-editor-description',
		defaultMessage: 'Content, files, backups and settings — everything except deleting the server.',
	},
	viewerRole: { id: 'craftpanel.access.role-viewer', defaultMessage: 'Limited' },
	viewerDescription: {
		id: 'craftpanel.access.role-viewer-description',
		defaultMessage: 'Watch the console and start or stop the server. No changes.',
	},
})

export const ROLE_LABELS: Record<ServerRole, MessageDescriptor> = {
	owner: roleMessages.ownerRole,
	editor: roleMessages.editorRole,
	viewer: roleMessages.viewerRole,
}

export const ROLE_DESCRIPTIONS: Partial<Record<ServerRole, MessageDescriptor>> = {
	editor: roleMessages.editorDescription,
	viewer: roleMessages.viewerDescription,
}

export const ROLE_COLORS: Record<ServerRole, string> = {
	owner: 'orange',
	editor: 'green',
	viewer: 'blue',
}
