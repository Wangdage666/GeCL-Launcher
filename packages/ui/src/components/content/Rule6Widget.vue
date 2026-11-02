<template>
	<div
		id="rule-6"
		class="rounded-2xl border border-solid border-surface-4 bg-surface-3 markdown-body p-4"
	>
		<nuxt-link to="/legal/rules#generative-ai" target="_blank">
			<h2 class="!mt-0">6. 生成式 AI 的使用</h2>
		</nuxt-link>

		<p> 项目必须如实披露制作、发布和项目功能中对生成式 AI 的使用。项目不得全部或主要由生成式 AI 的输出及其衍生内容组成。 </p>

		<h3 id="disclosure-of-ai-generated-content">6.1. AI 生成内容的披露</h3>

		<p>出现以下情况时，必须添加相应的“包含 AI 生成内容”声明：</p>
		<ol type="a">
			<li>项目的大量代码由 AI 生成。</li>
			<li> 项目包含主要或完全由 AI 生成的素材。 </li>
			<li>项目的设计或功能依赖生成式 AI。</li>
			<li> 项目页面中的介绍或发布内容等使用了生成式 AI。 </li>
		</ol>

		<h3 id="prohibited-usage-of-ai">6.2. 禁止的 AI 生成内容</h3>

		<ol type="a">
			<li> 上传到图库、图标、介绍及项目页面其他位置的图片，不得由生成式 AI 生成或由其输出衍生；此类图片可能被移除。 </li>
			<li> 内容主要或完全由 AI 生成的项目不得公开发布。 </li>
		</ol>
	</div>
</template>
<style scoped>
.markdown-body h2 {
	@apply mb-3 mt-5 pb-1 pt-0 text-xl font-semibold text-contrast;
}
.markdown-body h3 {
	@apply mb-1 mt-5 p-0 text-lg font-semibold text-contrast;
}

.markdown-body a {
	@apply !font-semibold !text-brand !no-underline hover:!text-brand hover:!underline;
}

.markdown-body p {
	@apply leading-normal;
}

.markdown-body ol {
	@apply mb-3;
}

.markdown-body ol ol {
	@apply mb-2;
}

.markdown-body li {
	@apply mb-1;
}
</style>
