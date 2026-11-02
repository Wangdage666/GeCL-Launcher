import {
	AuthFeature,
	NodeAuthFeature,
	nodeAuthState,
	PanelVersionFeature,
	TauriModrinthClient,
	VerboseLoggingFeature,
} from '@modrinth/api-client'
import { getVersion } from '@tauri-apps/api/app'

import { config } from '@/config'
import { get as getCreds } from '@/helpers/mr_auth'

export function createAppClient(): TauriModrinthClient {
	const appVersion = getVersion()

	return new TauriModrinthClient({
		userAgent: async () => `gecl/${await appVersion} (local desktop fork)`,
		labrinthBaseUrl: config.labrinthBaseUrl,
		archonBaseUrl: config.archonBaseUrl,
		sharedInstancesBaseUrl: config.sharedInstancesBaseUrl,
		features: [],
	})
}
