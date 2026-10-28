import type { Component } from 'vue'

import type { PanelSettings } from '@/api'
import JavaSettings from '@/pages/admin/settings/Java.vue'
import PortSettings from '@/pages/admin/settings/Ports.vue'
import RegistrationSettings from '@/pages/admin/settings/Registration.vue'

export interface PanelSettingsSection {
	id: string
	component: Component
	valid?: (settings: PanelSettings) => boolean
}

export const panelSettingsSections: PanelSettingsSection[] = [
	{ id: 'ports', component: PortSettings },
	{ id: 'registration', component: RegistrationSettings },
	{ id: 'java', component: JavaSettings },
]

export function sectionsValid(settings: PanelSettings): boolean {
	return panelSettingsSections.every((section) => section.valid?.(settings) ?? true)
}
