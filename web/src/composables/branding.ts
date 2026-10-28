import { readonly, ref } from 'vue'

import { api, type BrandingView } from '@/api'

const FALLBACK: BrandingView = { name: 'CraftPanel', custom_name: false, logo_url: null }
const FAVICON = '/favicon.svg'

const branding = ref<BrandingView>(FALLBACK)
let started = false

function apply(view: BrandingView): void {
	document.title = view.name
	let icon = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
	if (!icon) {
		icon = document.createElement('link')
		icon.rel = 'icon'
		document.head.appendChild(icon)
	}
	icon.href = view.logo_url ?? FAVICON
	icon.removeAttribute('type')
}

export function adoptBranding(view: BrandingView): void {
	branding.value = view
	apply(view)
}

export async function refreshBranding(): Promise<void> {
	try {
		adoptBranding(await api.branding.get())
	} catch {
		apply(branding.value)
	}
}

export function startBranding(): void {
	if (started) return
	started = true
	void refreshBranding()
}

export function useBranding() {
	return { branding: readonly(branding), refresh: refreshBranding, adopt: adoptBranding }
}
