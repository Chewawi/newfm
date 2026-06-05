import type { PlasmoCSConfig } from "plasmo";

export const config: PlasmoCSConfig = {
	matches: ["https://music.youtube.com/*"],
	all_frames: false,
};

// Keep track of the current video element we are observing
let observedVideo: HTMLVideoElement | null = null;

function getTrackMetadata() {
	let title = "";
	let artist = "";
	let album: string | null = null;

	// 1. Try MediaSession API (highly accurate on YT Music)
	if (navigator.mediaSession?.metadata) {
		const meta = navigator.mediaSession.metadata;
		title = meta.title || "";
		artist = meta.artist || "";
		album = meta.album || null;
	}

	// 2. Fallback to DOM parsing
	if (!title) {
		const titleEl = document.querySelector(
			".middle-controls .title, ytmusic-player-bar .title",
		);
		title = titleEl?.textContent?.trim() || "";
	}

	if (!artist) {
		const bylineEl = document.querySelector(
			".middle-controls .byline, ytmusic-player-bar .byline",
		);
		if (bylineEl) {
			const bylineText = bylineEl.textContent?.trim() || "";
			const parts = bylineText.split("•").map((p) => p.trim());
			artist = parts[0] || "";
			if (parts[1]) {
				album = parts[1];
			}
		}
	}

	return { title, artist, album };
}

function sendStateUpdate(video: HTMLVideoElement) {
	const metadata = getTrackMetadata();

	// Do not send update if there's no title or artist (i.e. not loaded yet)
	if (!metadata.title || !metadata.artist) {
		return;
	}

	chrome.runtime
		.sendMessage({
			type: "YTM_STATE",
			payload: {
				title: metadata.title,
				artist: metadata.artist,
				album: metadata.album,
				duration: video.duration || 0,
				currentTime: video.currentTime || 0,
				isPlaying: !video.paused && !video.ended,
			},
		})
		.catch((err) => {
			// Catch extension context invalidated errors when extension is reloaded
			console.debug("[newfm] Failed to send state update:", err);
		});
}

function setupVideoListeners(video: HTMLVideoElement) {
	if (observedVideo === video) return;

	// Remove listeners if any (though typically video elements are replaced or reused)
	observedVideo = video;

	const events = [
		"play",
		"pause",
		"timeupdate",
		"durationchange",
		"seeked",
		"ended",
	];
	events.forEach((eventName) => {
		video.addEventListener(eventName, () => sendStateUpdate(video));
	});

	console.log("[newfm] Connected to YouTube Music video player element.");
}

// Periodically check for the video element and verify state (handles SPA page transitions)
setInterval(() => {
	const video = document.querySelector("video");
	if (video) {
		setupVideoListeners(video);
		// Periodically send updates as a heartbeat backup
		sendStateUpdate(video);
	}
}, 1500);
