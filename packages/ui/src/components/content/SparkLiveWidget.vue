<script setup lang="ts">
import { VideoIcon } from '@modrinth/assets'

withDefaults(
	defineProps<{
		embed?: boolean
	}>(),
	{
		embed: false,
	},
)
</script>

<template>
	<div class="rounded-2xl border border-solid border-surface-4 bg-surface-3 overflow-hidden">
		<div class="p-4 flex flex-col gap-3">
			<div class="flex items-center gap-2">
				<VideoIcon class="size-6 text-brand" />
				<span class="text-contrast font-medium"> 我们今天稍后将与 Spark 进行直播交流，解答大家的问题。 </span>
			</div>
			<span> 可在我们的 <a
					href="https://discord.modrinth.com"
					target="_blank"
					class="text-brand font-semibold hover:underline"
					>Discord 服务器中提问</a
				>.
			</span>
			<span
				>直播时间为 6 月 15 日，原文标注 11:00 PST／14:00 EST／19:00 BST／20:00 CEST，观看地址： <a
					href="https://www.youtube.com/live/p1Dg-fud0TQ"
					target="_blank"
					class="text-brand font-semibold hover:underline"
					>YouTube 频道</a
				>.</span
			>
		</div>
		<div
			v-if="embed"
			style="left: 0; width: 100%; height: 0; position: relative; padding-bottom: 56.25%"
		>
			<iframe
				src="https://www.youtube.com/embed/p1Dg-fud0TQ"
				style="top: 0; left: 0; width: 100%; height: 100%; position: absolute; border: 0"
				allowfullscreen
				scrolling="no"
				allow="
					accelerometer *;
					clipboard-write *;
					encrypted-media *;
					gyroscope *;
					picture-in-picture *;
					web-share *;
				"
				referrerpolicy="strict-origin"
			></iframe>
		</div>
	</div>
</template>
