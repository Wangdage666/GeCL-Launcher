/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

export type ModrinthCredentials = {
	session: string
	expires: string
	user_id: string
	active: boolean
}

export type ModrinthAuthFlow = 'sign-in' | 'sign-up'

export async function login(
	flow: ModrinthAuthFlow = 'sign-in',
	addAccount = false,
): Promise<ModrinthCredentials> {
	throw new Error('gecl 不提供 Modrinth 账户服务')
}

export async function logout(): Promise<void> {
	return await invoke('plugin:mr-auth|logout')
}

export async function get(): Promise<ModrinthCredentials | null> {
	return null
}

export async function getAll(): Promise<ModrinthCredentials[]> {
	return []
}

export async function setActive(userId: string): Promise<void> {
	return await invoke('plugin:mr-auth|set_active', { userId })
}

export async function removeUser(userId: string): Promise<void> {
	return await invoke('plugin:mr-auth|remove_account', { userId })
}

export async function cancelLogin(): Promise<void> {
	return await invoke('plugin:mr-auth|cancel_modrinth_login')
}
