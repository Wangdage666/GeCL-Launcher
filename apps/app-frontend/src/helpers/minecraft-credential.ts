/** Public account data returned by the desktop bridge. Tokens stay in Rust. */
export interface MinecraftCredential {
	account_type: 'microsoft' | 'offline'
	profile: { id: string; name: string }
	expires: string
	active: boolean
}

export interface MicrosoftDeviceLogin {
	id: string
	user_code: string
	verification_uri: string
	expires_in: number
	interval: number
}
export type MicrosoftDevicePoll =
	| { status: 'pending'; interval: number }
	| { status: 'cancelled' }
	| { status: 'complete'; user: MinecraftCredential }
