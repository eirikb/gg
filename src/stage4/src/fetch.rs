use log::warn;
use serde::de::DeserializeOwned;
use std::sync::LazyLock;
use std::time::Duration;

/// A refused connection is quick, but a host that just swallows packets would leave
/// gg spinning forever - worse than the panic this replaced.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_default()
});

/// GET a URL and hand back the body. Someone else's index being down is a bad day, not
/// a reason to panic. Warn level, so the reason shows up without -v.
pub async fn fetch_text(url: &str) -> Option<String> {
    let response = match CLIENT.get(url).send().await {
        Ok(response) => response,
        Err(e) => {
            warn!("Could not reach {url}: {e}");
            return None;
        }
    };

    // Without this a 503 holding page is just a body, and the scrapers quietly
    // parse zero links out of it
    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(e) => {
            warn!("{url} answered with {e}");
            return None;
        }
    };

    match response.text().await {
        Ok(text) => Some(text),
        Err(e) => {
            warn!("Could not read the response from {url}: {e}");
            None
        }
    }
}

/// Same deal for the endpoints handing back JSON.
pub async fn fetch_json<T: DeserializeOwned>(url: &str) -> Option<T> {
    let text = fetch_text(url).await?;

    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            warn!("{url} did not answer with the JSON we expected: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Port 1 refuses right away, so this is the dead-host case without the wait
    const DEAD: &str = "http://127.0.0.1:1/index.json";

    #[tokio::test]
    async fn test_fetch_text_returns_none_when_the_host_is_dead() {
        assert_eq!(fetch_text(DEAD).await, None);
    }

    #[tokio::test]
    async fn test_fetch_json_returns_none_when_the_host_is_dead() {
        assert_eq!(fetch_json::<Vec<String>>(DEAD).await, None);
    }

    /// Answers one request, then goes away. Local, so the tests do not lean on
    /// somebody else's uptime - the thing this module exists to survive.
    async fn one_shot_server(status_line: &'static str, body: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                // Read the request off the socket before answering. Closing with
                // bytes still unread gets us an RST on Windows, and the response
                // we just wrote is thrown away with it
                let mut request = [0u8; 2048];
                let _ = socket.read(&mut request).await;
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
            }
        });
        format!("http://{addr}/index.json")
    }

    // A 503 holding page used to come back as a perfectly good body
    #[tokio::test]
    async fn test_fetch_text_returns_none_on_error_status() {
        let url = one_shot_server("503 Service Unavailable", "<html>maintenance</html>").await;
        assert_eq!(fetch_text(&url).await, None);
    }

    #[tokio::test]
    async fn test_fetch_text_returns_the_body_on_200() {
        let url = one_shot_server("200 OK", "hello").await;
        assert_eq!(fetch_text(&url).await.as_deref(), Some("hello"));
    }

    #[tokio::test]
    async fn test_fetch_json_returns_none_on_junk_body() {
        let url = one_shot_server("200 OK", "<html>not json</html>").await;
        assert_eq!(fetch_json::<Vec<String>>(&url).await, None);
    }
}
