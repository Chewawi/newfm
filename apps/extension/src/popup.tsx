// biome-ignore lint/correctness/noUnusedImports: si
import React, { useEffect, useState } from "react";
import "./popup.css";

interface TrackState {
	title: string;
	artist: string;
	album: string | null;
	duration: number;
	currentTime: number;
	isPlaying: boolean;
	startedAt: number;
	listenedMs: number;
	nowPlayingSent: boolean;
	scrobbled: boolean;
}

function IndexPopup() {
	const [activeTab, setActiveTab] = useState<"player" | "settings">("player");
	const [track, setTrack] = useState<TrackState | null>(null);

	// Settings Form States
	const [apiUrl, setApiUrl] = useState("");
	const [apiToken, setApiToken] = useState("");
	const [saveStatus, setSaveStatus] = useState<{
		success?: boolean;
		message?: string;
	} | null>(null);
	const [isVerifying, setIsVerifying] = useState(false);
	const [currentUser, setCurrentUser] = useState<string | null>(null);

	// Fetch current playback state and load settings
	useEffect(() => {
		// 1. Fetch the current playback state immediately
		chrome.runtime.sendMessage({ type: "GET_CURRENT_STATE" }, (response) => {
			if (response?.state) {
				setTrack(response.state);
			}
		});

		// 2. Poll current playback state every second
		const interval = setInterval(() => {
			chrome.runtime.sendMessage({ type: "GET_CURRENT_STATE" }, (response) => {
				if (response?.state) {
					setTrack(response.state);
				} else {
					setTrack(null);
				}
			});
		}, 1000);

		// 3. Load settings from storage
		chrome.storage.local.get(["apiUrl", "apiToken"], (items) => {
			setApiUrl(items.apiUrl || "http://localhost:8080");
			setApiToken(items.apiToken || "");
		});

		// 4. Verify existing connection
		chrome.runtime.sendMessage({ type: "VERIFY_CONNECTION" }, (res) => {
			if (res?.success && res.tokens && res.tokens.length > 0) {
				// Just extract the user/token context if verified
				setCurrentUser("API Active");
			} else {
				setCurrentUser(null);
			}
		});

		return () => clearInterval(interval);
	}, []);

	const handleSaveSettings = () => {
		setIsVerifying(true);
		setSaveStatus(null);

		chrome.runtime.sendMessage(
			{
				type: "SAVE_SETTINGS",
				payload: { apiUrl, apiToken },
			},
			(res) => {
				setIsVerifying(false);
				if (res?.success) {
					setSaveStatus({
						success: true,
						message: "SETTINGS SAVED + VERIFIED!",
					});
					setCurrentUser("API Active");
				} else {
					setSaveStatus({
						success: false,
						message: res?.error || "VERIFICATION FAILED.",
					});
					setCurrentUser(null);
				}
			},
		);
	};

	// Calculate Progress values
	const progressPercent =
		track && track.duration > 0
			? Math.min((track.currentTime / track.duration) * 100, 100)
			: 0;

	// Calculate scrobble milestone (min of 50% or 4 minutes)
	const scrobbleThresholdSeconds = track
		? Math.min(track.duration / 2, 240)
		: 30;

	const scrobblePercent =
		track && track.duration > 0
			? (scrobbleThresholdSeconds / track.duration) * 100
			: 50;

	const formatTime = (secs: number) => {
		if (Number.isNaN(secs) || secs <= 0) return "0:00";
		const m = Math.floor(secs / 60);
		const s = Math.floor(secs % 60);
		return `${m}:${s < 10 ? "0" : ""}${s}`;
	};

	return (
		<div className="popup-container">
			<div className="header">
				<h1 className="title">NEW.FM</h1>
				<div
					className={`status-dot ${currentUser ? "connected" : "disconnected"}`}
					title={currentUser ? `Connected (${currentUser})` : "Disconnected"}
				/>
			</div>

			<div className="tabs-header">
				<button
					type="button"
					className={`tab-btn ${activeTab === "player" ? "active" : ""}`}
					onClick={() => setActiveTab("player")}
				>
					Player
				</button>
				<button
					type="button"
					className={`tab-btn ${activeTab === "settings" ? "active" : ""}`}
					onClick={() => setActiveTab("settings")}
				>
					Settings
				</button>
			</div>

			{activeTab === "player" ? (
				<div className="tab-content">
					{track ? (
						<>
							<div>
								<h2 className="song-title">{track.title}</h2>
								<div className="song-artist">{track.artist}</div>
								<div className="song-album">{track.album || "[NO ALBUM]"}</div>
							</div>

							<div className="progress-box">
								<div className="progress-bar-container">
									{/* Mark the scrobble threshold line */}
									<div
										className="scrobble-marker"
										style={{ left: `${scrobblePercent}%` }}
										title="Scrobble point"
									/>
									<div
										className="progress-bar-fill"
										style={{ width: `${progressPercent}%` }}
									/>
								</div>
								<div className="time-labels">
									<span>{formatTime(track.currentTime)}</span>
									<span>{formatTime(track.duration)}</span>
								</div>
							</div>

							{track.scrobbled ? (
								<div className="status-badge scrobbled">SCROBBLED [OK]</div>
							) : track.nowPlayingSent ? (
								<div className="status-badge nowplaying">NOW PLAYING</div>
							) : (
								<div className="status-badge">TUNING IN...</div>
							)}
						</>
					) : (
						<div className="empty-state">
							SILENCE IN THE AIRWAVES.
							<br />
							<br />
							<span style={{ color: "var(--neon-pink)" }}>PLAY SOMETHING</span>
							<br />
							ON YOUTUBE MUSIC.
						</div>
					)}
				</div>
			) : (
				<div className="tab-content settings-view">
					<div>
						<div className="form-group">
							<label htmlFor="api-url" className="form-label">
								API BASE URL
							</label>
							<input
								type="text"
								className="form-input"
								value={apiUrl}
								onChange={(e) => setApiUrl(e.target.value)}
								placeholder="http://localhost:8080"
							/>
						</div>

						<div className="form-group">
							<label htmlFor="api-token" className="form-label">
								API TOKEN
							</label>
							<input
								type="password"
								className="form-input"
								value={apiToken}
								onChange={(e) => setApiToken(e.target.value)}
								placeholder="Enter API token..."
							/>
						</div>
					</div>

					<div>
						<button
							type="button"
							className="btn-submit"
							onClick={handleSaveSettings}
							disabled={isVerifying}
						>
							{isVerifying ? "VERIFYING..." : "SAVE & CONNECT"}
						</button>

						{saveStatus && (
							<div
								className={`status-msg ${saveStatus.success ? "success" : "error"}`}
							>
								{saveStatus.message}
							</div>
						)}
					</div>
				</div>
			)}
		</div>
	);
}

export default IndexPopup;
