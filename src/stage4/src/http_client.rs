use log::{info, warn};
use reqwest::Certificate;
use std::fs::{create_dir_all, File};
use std::io::Write;

// You might look at this file and think "why not use the system CA certificates directly?".
// And you are absolutely right! However, this is a fallback mechanism. I promise.
// On some systems, especially in Docker containers or minimal installations,
// the system CA certificates might be outdated or missing, leading to SSL errors.
// This code attempts to download a fresh set of CA certificates from curl.se and use them.
// Note note note; this still relies on the CA for curl.se being valid and trusted.
// And it is only a fallback mechanism.
// See https://github.com/eirikb/gg/issues/119

const CA_CERT_URL: &str = "https://curl.se/ca/cacert.pem";
const CA_CERT_PATH: &str = ".cache/gg/cacert.pem";

pub async fn create_http_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    match reqwest::Client::builder().build() {
        Ok(client) => Ok(client),
        Err(e) => {
            if is_cert_error(&e.to_string()) {
                handle_cert_error(&format!(
                    "Failed to create HTTP client due to certificate issues: {}",
                    e
                ))
                .await?;

                match reqwest::Client::builder().build() {
                    Ok(client) => {
                        warn!("Successfully created HTTP client with fallback CA certificates!");
                        Ok(client)
                    }
                    Err(_) => create_client_with_ca_certs().await,
                }
            } else {
                Err(e.into())
            }
        }
    }
}

async fn download_ca_certs() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let res = client.get(CA_CERT_URL).send().await?;
    let ca_cert_content = res.bytes().await?;

    create_dir_all(".cache/gg")?;

    let mut file = File::create(CA_CERT_PATH)?;
    file.write_all(&ca_cert_content)?;

    info!("Downloaded CA certificates to {}", CA_CERT_PATH);
    Ok(())
}

async fn create_client_with_ca_certs() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let ca_cert_content = tokio::fs::read(CA_CERT_PATH).await?;
    let cert = Certificate::from_pem(&ca_cert_content)?;

    let client = reqwest::Client::builder()
        .add_root_certificate(cert)
        .build()?;

    Ok(client)
}

fn is_cert_error(error_str: &str) -> bool {
    error_str.contains("native root CA certificates")
        || error_str.contains("certificate")
        || error_str.contains("SSL")
        || error_str.contains("TLS")
}

async fn handle_cert_error(context: &str) -> Result<(), Box<dyn std::error::Error>> {
    warn!("\n\n=== WARNING: CA CERTIFICATE ERROR DETECTED ===");
    warn!("{}", context);
    warn!(
        "Attempting to download CA certificates from {}...",
        CA_CERT_URL
    );
    warn!(
        "This is a FALLBACK mechanism - your system's CA certificates may be outdated or missing!"
    );
    warn!("Consider updating your system's CA certificates for better security.\n");

    download_ca_certs().await?;

    std::env::set_var("SSL_CERT_FILE", CA_CERT_PATH);
    warn!(
        "Set SSL_CERT_FILE environment variable to {}. Retrying...",
        CA_CERT_PATH
    );

    Ok(())
}

pub async fn create_octocrab_instance(
    base_uri: &str,
) -> Result<octocrab::Octocrab, Box<dyn std::error::Error>> {
    match octocrab::Octocrab::builder().base_uri(base_uri)?.build() {
        Ok(client) => Ok(client),
        Err(e) => {
            let error_str = format!("{:?}", e);
            if is_cert_error(&error_str) {
                handle_cert_error(&format!(
                    "Failed to create GitHub API client: {}",
                    error_str
                ))
                .await?;

                match octocrab::Octocrab::builder().base_uri(base_uri)?.build() {
                    Ok(client) => {
                        warn!(
                            "Successfully created GitHub API client with fallback CA certificates!"
                        );
                        Ok(client)
                    }
                    Err(e2) => {
                        warn!("Failed to create GitHub API client even with fallback certificates: {}", e2);
                        Err(e2.into())
                    }
                }
            } else {
                Err(e.into())
            }
        }
    }
}
