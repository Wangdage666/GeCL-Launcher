export interface MinecraftAuthError {
	whatHappened: string
	stepsToFix: string[]
}

const networkError: MinecraftAuthError = {
	whatHappened: '无法连接登录所需的 Microsoft、Xbox 或 Minecraft 服务。请检查网络、DNS、代理、防火墙、hosts 文件及安全软件。',
	stepsToFix: ['检查互联网连接后重新登录', '检查防火墙、安全软件、代理、VPN 和 hosts 规则是否阻止了相关服务', '尝试其他网络，或临时关闭正在使用的代理、VPN 后重试'],
}

export const minecraftAuthErrors: Record<string, MinecraftAuthError> = {
	minecraft_auth_token_expired: {
		whatHappened: '保存的微软登录凭据已过期或被撤销，无法刷新 Minecraft 会话。',
		stepsToFix: ['移除受影响的微软账户', '重新添加并登录该账户', '登录完成后重新启动游戏'],
	},
	minecraft_auth_clock_out_of_sync: {
		whatHappened: 'Xbox 拒绝了登录响应，系统时间或时区不准确可能导致此问题。',
		stepsToFix: ['打开系统日期与时间设置', '启用自动设置时间和时区，并立即同步时间', '重启 gecl 后重新登录'],
	},
	minecraft_auth_rate_limited: {
		whatHappened: '近期登录请求过多，Microsoft 或 Minecraft 暂时限制了请求。',
		stepsToFix: ['暂停重复尝试，稍后再登录', '如果仍被限制，请延长等待时间'],
	},
	minecraft_auth_service_unavailable: {
		whatHappened: 'Minecraft 认证服务返回服务器错误，暂时无法完成登录。',
		stepsToFix: ['稍等几分钟后重试', '查看 <a href="https://support.xbox.com/xbox-live-status">Xbox 服务状态</a>', '尝试使用 Minecraft 官方启动器登录，以判断服务是否普遍异常'],
	},
	minecraft_auth_no_profile: {
		whatHappened: '未能取得此账户的 Java 版玩家资料。可能刚购买游戏、尚未创建玩家资料，或登录了其他账户。',
		stepsToFix: ['登录 <a href="https://www.minecraft.net/en-us/download">Minecraft 官方启动器</a>，运行一次 Java 版', '若刚购买游戏或创建资料，请等待资料同步', '确认使用的是拥有游戏的微软账户，再回到 gecl 登录'],
	},
	minecraft_auth_network: networkError,
	minecraft_auth_connection_refused: networkError,
	minecraft_auth_age_verification: {
		whatHappened: '账户需要完成适用地区要求的年龄验证。',
		stepsToFix: ['前往 <a href="https://www.minecraft.net/en-us/login">Minecraft 登录页</a>登录', '按页面提示完成年龄验证后重试'],
	},
	minecraft_auth_no_xbox_profile: {
		whatHappened: '此账户尚未设置 Xbox 个人资料，或没有 Minecraft 拥有权。',
		stepsToFix: ['确认此账户已购买 Minecraft', '登录 Minecraft 网站，按提示完成 Xbox 个人资料设置', '完成后重新登录'],
	},
	minecraft_auth_region_unavailable: {
		whatHappened: 'Xbox Live 在账户所在地区不可用，登录被阻止。',
		stepsToFix: ['在 <a href="https://www.xbox.com/en-US/regions">Xbox 服务地区列表</a>中核实支持范围'],
	},
	minecraft_auth_adult_verification: {
		whatHappened: '此账户需要完成韩国地区要求的成人身份验证。',
		stepsToFix: ['登录 <a href="https://www.xbox.com">Xbox 网站</a>', '完成身份验证后重新登录'],
	},
	minecraft_auth_underage: {
		whatHappened: '此账户未成年，且尚未加入微软家庭组。',
		stepsToFix: ['请监护人参阅 <a href="https://help.minecraft.net/hc/en-us/articles/4408968616077">家庭设置指南</a>', '按要求加入或创建家庭组，完成后重试'],
	},
	minecraft_auth_banned: {
		whatHappened: '此账户因违反 Xbox 社区准则而被停用。',
		stepsToFix: ['前往 <a href="https://support.xbox.com">Xbox 支持</a>查看处罚详情', '若提供申诉入口，可按说明提交申诉'],
	},
	minecraft_auth_restricted: {
		whatHappened: '此账户受到限制，没有在线游玩的权限。',
		stepsToFix: ['请监护人登录 <a href="https://account.microsoft.com/family/">微软家庭</a>', '检查在线游玩权限，调整后重新登录'],
	},
	minecraft_auth_terms_not_accepted: {
		whatHappened: '此账户尚未接受 Xbox 服务条款。',
		stepsToFix: ['登录 <a href="https://www.xbox.com">Xbox 网站</a>', '按提示接受条款后重试'],
	},
	minecraft_auth_xsts_rejected: {
		whatHappened: 'Xbox 拒绝了 Minecraft 服务授权请求，但没有返回可识别的具体账户限制。',
		stepsToFix: ['使用 Minecraft 官方启动器登录，并完成 Microsoft、Xbox 或 Minecraft 的提示步骤', '回到 gecl 重新登录', '若官方启动器也失败，请按其错误提示处理，或联系 Xbox 支持'],
	},
}
