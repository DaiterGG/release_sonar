const CLIENT_ID = "5816d7f999ca4a7390e154dbf20eee5b";
const URI = "https://daitergg.github.io/release_sonar";
const REDIRECT_URI = URI + "/callback";
const SCOPE = "user-library-read";
const SERVER_URL =
  "https://0tqhj2esqh.execute-api.eu-north-1.amazonaws.com/Prod/";
const SERVER_URL_POLL = SERVER_URL + "/poll";
const STATE_CHARSET =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const STATE_LENGTH = 16;
const app = document.getElementById("app");
// Helper: generate random string
function generateRandomString(length, charset) {
  let result = "";
  const values = new Uint8Array(length);
  crypto.getRandomValues(values);
  for (let i = 0; i < length; i++) {
    result += charset[values[i] % charset.length];
  }
  return result;
}
function getQueryParams() {
  const params = new URLSearchParams(window.location.search);
  const obj = {};
  for (const [key, value] of params) {
    obj[key] = value;
  }
  return obj;
}
// Login flow
function initiateLogin() {
  const state = generateRandomString(STATE_LENGTH, STATE_CHARSET);
  sessionStorage.setItem("spotify_auth_state", state);
  const authUrl = new URL("https://accounts.spotify.com/authorize");
  authUrl.searchParams.set("response_type", "code");
  authUrl.searchParams.set("client_id", CLIENT_ID);
  authUrl.searchParams.set("scope", SCOPE);
  authUrl.searchParams.set("redirect_uri", REDIRECT_URI);
  authUrl.searchParams.set("state", state);
  window.location.href = authUrl.toString();
}
// Callback after Spotify redirect
async function handleCallback() {
  const params = getQueryParams();
  if (params.error) {
    displayError(`Spotify returned an error: ${params.error}`);
    return;
  }
  const storedState = sessionStorage.getItem("spotify_auth_state");
  if (!storedState) {
    displayError(
      "No state found in session storage. Possible CSRF or login not initiated from this browser.",
    );
    return;
  }
  if (storedState !== params.state) {
    displayError("State mismatch. Possible CSRF attack.");
    return;
  }
  sessionStorage.removeItem("spotify_auth_state");
  if (!params.code) {
    displayError("No authorization code returned.");
    return;
  }
  const code = params.code;
  const time = Date.now();
  try {
    const minTracksInput = document.getElementById("minTracks");
    const minTracks = minTracksInput ? minTracksInput.value : "2";
    const response = await fetch(SERVER_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code, time, min_tracks: minTracks }),
    });
    if (!response.ok) {
      throw new Error(`Backend returned ${response.status}`);
    }
    // Store credentials and login flag
    sessionStorage.setItem("spotify_auth_state", code);
    sessionStorage.setItem("spotify_expire_time", time.toString());
    sessionStorage.setItem("spotify_logged_in", "true");
    // Redirect back to main page
    window.location.href = URI;
  } catch (error) {
    displayError("Failed to exchange code. Please try again.");
    console.error("Exchange error:", error);
  }
}
// Polling function (triggered by search button)
async function startPolling() {
  const code = sessionStorage.getItem("spotify_auth_state");
  const time = sessionStorage.getItem("spotify_expire_time");
  if (!code || !time) {
    displayError("Missing login data. Please log in again.");
    return;
  }
  app.innerHTML = `
    <div class="spinner"></div>
    <p class="progress">Loading your new releases…</p>
    <p class="progress">This could take a couple minutes</p>
  `;
  const intervalId = setInterval(async () => {
    try {
      const response = await fetch(SERVER_URL_POLL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          code,
          time,
        }),
      });
      if (!response.ok) {
        throw new Error(`Backend returned ${response.status}`);
      }
      const data = await response.json();
      if (data.job_state === "PROGRESS") {
        app.innerHTML =
          `<div class="progress">⏳ In Progress: ${data.job_result}</div>`;
      }
      if (data.job_state === "DONE") {
        clearInterval(intervalId);
        sessionStorage.removeItem("spotify_auth_state");
        sessionStorage.removeItem("spotify_expire_time");
        // Keep login flag so search button stays enabled
        try {
          const resultObj = JSON.parse(data.job_result);
          renderTracks(resultObj.tracks);
        } catch (e) {
          app.innerHTML =
            `<div class="error">Failed to parse track data.</div>`;
        }
      }
    } catch (error) {
      console.error("Polling error:", error);
      app.innerHTML =
        `<div class="error">⚠️ Polling failed – check connection</div>`;
      clearInterval(intervalId);
      sessionStorage.removeItem("spotify_auth_state");
      sessionStorage.removeItem("spotify_expire_time");
    }
  }, 10000);
}
// Render tracks
function renderTracks(tracks) {
  if (!tracks || tracks.length === 0) {
    app.innerHTML = `<div class="result">No new tracks found.</div>`;
    return;
  }
  const trackItems = tracks.map((track) => {
    // Clean up trailing comma and space (intentional spaces)
    const artists = track.artists.replace(/, $/, "").replace(/,  /g, ", ");
    return `
      <li class="track-item">
        <div class="track-name">${escapeHTML(track.name)}</div>
        <div class="track-artists">${escapeHTML(artists)}</div>
        <div class="track-date">${escapeHTML(track.date)}</div>
      </li>
    `;
  }).join("");
  app.innerHTML = `
    <h2 style="margin-bottom: 1rem; color: #fff;">🎵 New Releases</h2>
    <ul class="track-list">${trackItems}</ul>
  `;
}
// Simple escape
function escapeHTML(str) {
  return str.replace(/[&<>"]/g, function (m) {
    if (m === "&") return "&amp;";
    if (m === "<") return "&lt;";
    if (m === ">") return "&gt;";
    if (m === '"') return "&quot;";
    return m;
  });
}
function displayError(message) {
  console.error(message);
  app.innerHTML = `<div class="error">❌ ${message}</div>`;
}
// ----- Routing -----
if (window.location.pathname.includes("callback")) {
  handleCallback();
} else {
  // ----- Main page setup -----
  const loginBtn = document.getElementById("login-button");
  const searchBtn = document.getElementById("search-button");
  const minTracksInput = document.getElementById("minTracks");
  const minTracksSpan = document.getElementById("minTracksValue");
  // Update displayed slider value
  if (minTracksInput && minTracksSpan) {
    minTracksInput.addEventListener("input", () => {
      minTracksSpan.textContent = minTracksInput.value;
    });
  }
  // Enable/disable search button based on login status
  function updateSearchButtonState() {
    if (sessionStorage.getItem("spotify_logged_in") === "true") {
      searchBtn.disabled = false;
    } else {
      searchBtn.disabled = true;
    }
  }
  updateSearchButtonState();
  // Attach event listeners
  loginBtn.addEventListener("click", initiateLogin);
  searchBtn.addEventListener("click", startPolling);
}
