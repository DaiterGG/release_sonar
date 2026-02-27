const CLIENT_ID = '5816d7f999ca4a7390e154dbf20eee5b';
const URI = 'https://daitergg.github.io/release_sonar';
const REDIRECT_URI = URI + '/callback';
const SCOPE = 'user-library-read';

const SERVER_URL = 'https://0tqhj2esqh.execute-api.eu-north-1.amazonaws.com/Prod/';
const SERVER_URL_POLL = SERVER_URL + '/poll';

const STATE_CHARSET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
const STATE_LENGTH = 16;

function generateRandomString(length, charset) {
    let result = '';
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

function initiateLogin() {
    const state = generateRandomString(STATE_LENGTH, STATE_CHARSET);
    sessionStorage.setItem('spotify_auth_state', state);

    const storedState = sessionStorage.getItem('spotify_auth_state');

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
    const time = Date.now();

    try {
        const response = await fetch(SERVER_URL, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                code: code,
                time: time
            }),
        });
        if (!response.ok) {
            throw new Error(`Backend returned ${response.status}`);
        }

        sessionStorage.setItem('spotify_auth_state', code);
        sessionStorage.setItem('spotify_expire_time', time.toString());

        window.location.href = URI + '?q=start_polling';
    } catch (error) {
        document.body.innerHTML = `<div style="color: red; padding: 2rem;">Failed to exchange code</div>`;
        console.error('Polling error:', error);
    }
}

function startPolling() {
    const code = sessionStorage.getItem('spotify_auth_state');
    const time = sessionStorage.getItem('spotify_expire_time');

    if (!code || !time) return;

    const intervalId = setInterval(async () => {
        try {
            const response = await fetch(SERVER_URL_POLL, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    code: code,
                    time: time
                }),
            });
            if (!response.ok) {
                throw new Error(`Backend returned ${response.status}\n Try Again`);
                clearInterval(intervalId);
                sessionStorage.removeItem('spotify_auth_state');
                sessionStorage.removeItem('spotify_expire_time');
            }
            const data = await response.json();

            if (data.job_state == "PROGRESS" ) {
                document.body.innerHTML = `<div style="color: red; padding: 2rem;">In Progress: ${data.job_result}</div>`;
            }
            if (data.job_state == "DONE" ) {
                clearInterval(intervalId);
                sessionStorage.removeItem('spotify_auth_state');
                sessionStorage.removeItem('spotify_expire_time');
                document.body.innerHTML = `<div style="color: red; padding: 2rem;">Result: ${data.job_result}</div>`;
            }
        } catch (error) {
            console.error('Polling error:', error);
            // TODO: stop polling on persistent errors
        }
    }, 10000);
}


function displayError(message) {
    console.error(message);
    document.body.innerHTML = `<div style="color: red; padding: 2rem;">${message}</div>`;
}

if (window.location.pathname.includes('callback')) {
    handleCallback();
} else {
    document.getElementById('login-button').addEventListener('click', initiateLogin);

    params = new URLSearchParams(window.location.search);
    if (params.has("q")) {
        startPolling();
    }
}
