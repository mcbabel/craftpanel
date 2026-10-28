<template>
	<svg viewBox="104 82 304 348" class="craftpanel-logo" aria-hidden="true">
		<polygon class="face-top" points="256,92 398,174 256,256 114,174" />
		<polygon class="face-left" points="114,174 256,256 256,420 114,338" />
		<polygon class="face-right" points="256,256 398,174 398,338 256,420" />
		<g class="slots">
			<polygon points="136,214 234,270.6 234,288.6 136,232" />
			<polygon points="136,256 234,312.6 234,330.6 136,274" />
			<polygon points="136,298 234,354.6 234,372.6 136,316" />
		</g>
		<g class="leds">
			<circle cx="300" cy="286" r="10" />
			<circle cx="330" cy="268.5" r="10" opacity="0.55" />
			<circle cx="360" cy="251" r="10" opacity="0.3" />
		</g>
	</svg>
</template>

<style scoped>
.craftpanel-logo {
	display: block;
}

.face-top {
	fill: color-mix(in srgb, var(--color-brand) 62%, white);
}

.face-left {
	fill: var(--color-brand);
}

.face-right {
	fill: color-mix(in srgb, var(--color-brand) 72%, black);
}

.slots {
	fill: color-mix(in srgb, var(--color-brand) 30%, black);
	opacity: 0.6;
}

.leds {
	fill: color-mix(in srgb, var(--color-brand) 25%, white);
}
</style>
