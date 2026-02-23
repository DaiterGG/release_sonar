const CLIENT_ID = '5816d7f999ca4a7390e154dbf20eee5b';
const REDIRECT_URI = 'https://daitergg.github.io/release_sonar/callback';
const SCOPE = 'user-library-read';

const TOKEN_EXCHANGE_URL = 'https://0tqhj2esqh.execute-api.eu-north-1.amazonaws.com/Prod/request/';

const STATE_CHARSET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
const STATE_LENGTH = 16;

// Generate a random string of given length from charset
function generateRandomString(length, charset) {
    let result = '';
    const values = new Uint8Array(length);
    crypto.getRandomValues(values);
    for (let i = 0; i < length; i++) {
        result += charset[values[i] % charset.length];
    }
    return result;
}

// Parse query string into an object
function getQueryParams() {
    const params = new URLSearchParams(window.location.search);
    const obj = {};
    for (const [key, value] of params) {
        obj[key] = value;
    }
    return obj;
}

function initiateLogin() {
    const state = generateRandomString(STATE_LENGTH, STATE_CHARSET);
    sessionStorage.setItem('spotify_auth_state', state);

    const authUrl = new URL('https://accounts.spotify.com/authorize');
    authUrl.searchParams.set('response_type', 'code');
    authUrl.searchParams.set('client_id', CLIENT_ID);
    authUrl.searchParams.set('scope', SCOPE);
    authUrl.searchParams.set('redirect_uri', REDIRECT_URI);
    authUrl.searchParams.set('state', state);

    window.location.href = authUrl.toString();
}

async function handleCallback() {
    const params = getQueryParams();

    if (params.error) {
        displayError(`Spotify returned an error: ${params.error}`);
        return;
    }

    const storedState = sessionStorage.getItem('spotify_auth_state');
    if (!storedState) {
        displayError('No state found in session storage. Possible CSRF or login not initiated from this browser.');
        return;
    }
    if (storedState !== params.state) {
        displayError('State mismatch. Possible CSRF attack.');
        return;
    }

    sessionStorage.removeItem('spotify_auth_state');

    if (!params.code) {
        displayError('No authorization code returned.');
        return;
    }

    const code = params.code;

    try {
        const response = await fetch(TOKEN_EXCHANGE_URL, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: code,
        });
        if (!response.ok) {
            throw new Error(`Backend returned ${response.status}`);
        }
        window.location.href = '/release_sonar';
    } catch (error) {
        displayError(`Failed to exchange code: ${error.message}`);
    }
}

function displayError(message) {
    console.error(message);
    document.body.innerHTML = `<div style="color: red; padding: 2rem;">${message}</div>`;
}

if (window.location.pathname.includes('callback')) {
    handleCallback();
} else {
    document.getElementById('login-button').addEventListener('click', initiateLogin);
}
