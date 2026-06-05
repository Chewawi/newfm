import type { ApiToken } from "types";

interface TrackState {
	title: string;
	artist: string;
	album: string | null;
	duration: number; // in seconds
	currentTime: number; // in seconds
	isPlaying: boolean;
	startedAt: number; // timestamp in ms when track was first detected
	listenedMs: number; // cumulative ms listened to this track
	lastTickTime: number; // timestamp in ms of the last state update
	nowPlayingSent: boolean;
	scrobbled: boolean;
}

// Store playback state per tab
const tabStates = new Map<number, TrackState>();

// Helper to get configuration settings
async function getSettings(): Promise<{ apiUrl: string; apiToken: string }> {
	return new Promise((resolve) => {
		chrome.storage.local.get(["apiUrl", "apiToken"], (items) => {
			resolve({
				apiUrl: items.apiUrl || "http://localhost:8080",
				apiToken: items.apiToken || "",
			});
		});
	});
}

// Send current playing info to API
async function sendNowPlaying(state: TrackState) {
	const { apiUrl, apiToken } = await getSettings();
	if (!apiToken) {
		console.debug("[newfm] No API Token configured. Skipping now playing.");
		return;
	}

	try {
		const response = await fetch(`${apiUrl}/v1/now-playing`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${apiToken}`,
			},
			body: JSON.stringify({
				track: state.title,
				artist: state.artist,
				album: state.album,
				duration_ms:
					state.duration > 0 ? Math.round(state.duration * 1000) : null,
				source: "ytmusic",
			}),
		});

		if (!response.ok) {
			return new Error(`HTTP ${response.status}: ${await response.text()}`);
		}

		state.nowPlayingSent = true;
		console.log(`[newfm] Now playing sent: ${state.title} by ${state.artist}`);
	} catch (error) {
		console.error("[newfm] Failed to send now playing:", error);
	}
}

// Send completed scrobble to API
async function sendScrobble(state: TrackState) {
	const { apiUrl, apiToken } = await getSettings();
	if (!apiToken) {
		console.debug("[newfm] No API Token configured. Skipping scrobble.");
		return;
	}

	try {
		const response = await fetch(`${apiUrl}/v1/scrobble`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${apiToken}`,
			},
			body: JSON.stringify({
				track: state.title,
				artist: state.artist,
				album: state.album,
				played_at: new Date(state.startedAt).toISOString(),
				duration_ms:
					state.duration > 0 ? Math.round(state.duration * 1000) : null,
				listened_ms: Math.round(state.listenedMs),
				source: "ytmusic",
			}),
		});

		if (!response.ok) {
			return new Error(`HTTP ${response.status}: ${await response.text()}`);
		}

		state.scrobbled = true;
		console.log(
			`[newfm] Scrobbled successfully: ${state.title} by ${state.artist}`,
		);
	} catch (error) {
		console.error("[newfm] Failed to send scrobble:", error);
	}
}

// Test settings and return active token scopes or error
async function verifySettings(
	apiUrl: string,
	apiToken: string,
): Promise<{
	success: boolean;
	error?: string;
	tokens?: ApiToken[];
}> {
	try {
		const response = await fetch(`${apiUrl}/v1/auth/tokens`, {
			method: "GET",
			headers: {
				Authorization: `Bearer ${apiToken}`,
			},
		});

		if (response.ok) {
			const tokens = (await response.json()) as ApiToken[];
			return { success: true, tokens };
		} else if (response.status === 401) {
			return { success: false, error: "Invalid API Token (Unauthorized)" };
		} else {
			return { success: false, error: `Server error: ${response.statusText}` };
		}
		// biome-ignore lint/suspicious/noExplicitAny: ok
	} catch (err: any) {
		return { success: false, error: `Connection failed: ${err.message}` };
	}
}

// Handle runtime messages
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
	const tabId = sender.tab?.id;

	if (message.type === "YTM_STATE" && tabId !== undefined) {
		const payload = message.payload;
		const now = Date.now();

		let state = tabStates.get(tabId);
		const isNewTrack =
			!state ||
			state.title !== payload.title ||
			state.artist !== payload.artist;

		if (isNewTrack) {
			// If we had a previous track that wasn't scrobbled, we could attempt to scrobble it here.
			// But we scrobble immediately when the threshold is hit anyway.

			state = {
				title: payload.title,
				artist: payload.artist,
				album: payload.album,
				duration: payload.duration,
				currentTime: payload.currentTime,
				isPlaying: payload.isPlaying,
				startedAt: now,
				listenedMs: 0,
				lastTickTime: now,
				nowPlayingSent: false,
				scrobbled: false,
			};
			tabStates.set(tabId, state);
			console.log(`[newfm] Track detected: ${state.title} by ${state.artist}`);
		} else {
			// Update accumulation
			if (state.isPlaying) {
				const delta = now - state.lastTickTime;
				// Sanity check to prevent large leaps if tab was asleep/throttled
				if (delta > 0 && delta < 5000) {
					state.listenedMs += delta;
				}
			}

			state.lastTickTime = now;
			state.isPlaying = payload.isPlaying;
			state.currentTime = payload.currentTime;
			state.duration = payload.duration;
			// YT Music can dynamically fill in album name or duration later in the track
			if (payload.album && !state.album) {
				state.album = payload.album;
			}
		}

		// Now Playing Threshold: 5 seconds of active listening
		if (!state.nowPlayingSent && state.listenedMs >= 5000) {
			// Set sent flag immediately to prevent overlapping API calls
			state.nowPlayingSent = true;
			sendNowPlaying(state).then();
		}

		// Scrobble Threshold:
		// 1. Min 30 seconds
		// 2. AND (Min 50% of track OR 4 minutes)
		const minListenMs = 30000;
		const durationMs = state.duration * 1000;
		const scrobblePercentMs = durationMs > 0 ? durationMs / 2 : minListenMs;
		const scrobbleMaxMs = 240000; // 4 minutes
		const thresholdMs = Math.min(scrobblePercentMs, scrobbleMaxMs);

		if (
			!state.scrobbled &&
			state.listenedMs >= minListenMs &&
			state.listenedMs >= thresholdMs
		) {
			// Set scrobbled flag immediately
			state.scrobbled = true;
			sendScrobble(state).then(() => void 0);
		}

		return false;
	}

	if (message.type === "GET_CURRENT_STATE") {
		// Find active tab state
		chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
			const activeTab = tabs[0];
			if (activeTab && activeTab.id !== undefined) {
				const state = tabStates.get(activeTab.id);
				sendResponse({ state: state || null });
			} else {
				sendResponse({ state: null });
			}
		});
		return true; // Keep channel open for async query
	}

	if (message.type === "SAVE_SETTINGS") {
		const { apiUrl, apiToken } = message.payload;
		verifySettings(apiUrl, apiToken).then((res) => {
			if (res.success) {
				chrome.storage.local.set({ apiUrl, apiToken }, () => {
					sendResponse({ success: true });
				});
			} else {
				sendResponse({ success: false, error: res.error });
			}
		});
		return true;
	}

	if (message.type === "VERIFY_CONNECTION") {
		getSettings().then(({ apiUrl, apiToken }) => {
			if (!apiToken) {
				sendResponse({ success: false, error: "API Token not configured" });
				return;
			}
			verifySettings(apiUrl, apiToken).then((res) => {
				sendResponse(res);
			});
		});
		return true;
	}
});
