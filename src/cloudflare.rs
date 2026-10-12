use std::{
    net::IpAddr,
    sync::{LazyLock, OnceLock},
    time::Duration,
};

use actix_web::{
    Error, HttpRequest, HttpResponse,
    body::{EitherBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http::header,
    middleware::Next,
};
use log::{info, warn};

const RANGES: &[&str] = &[
    "173.245.48.0/20",
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "141.101.64.0/18",
    "108.162.192.0/18",
    "190.93.240.0/20",
    "188.114.96.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "131.0.72.0/22",
    "2400:cb00::/32",
    "2606:4700::/32",
    "2803:f800::/32",
    "2405:b500::/32",
    "2405:8100::/32",
    "2a06:98c0::/29",
    "2c0f:f248::/32",
];

const LIST_URLS: [&str; 2] = [
    "https://www.cloudflare.com/ips-v4",
    "https://www.cloudflare.com/ips-v6",
];

static NETWORKS: OnceLock<Vec<(IpAddr, u32)>> = OnceLock::new();

static ALLOW_ANY: LazyLock<bool> =
    LazyLock::new(|| std::env::var("ALLOW_NON_CLOUDFLARE").is_ok_and(|v| v == "true"));

fn parse_range(range: &str) -> Option<(IpAddr, u32)> {
    let (ip, len) = range.trim().split_once('/')?;
    let ip: IpAddr = ip.parse().ok()?;
    let len: u32 = len.parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    (len <= max).then_some((ip, len))
}

fn builtin() -> Vec<(IpAddr, u32)> {
    RANGES
        .iter()
        .map(|range| parse_range(range).expect("builtin Cloudflare range is valid"))
        .collect()
}

async fn fetch() -> anyhow::Result<Vec<(IpAddr, u32)>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut networks = vec![];
    for url in LIST_URLS {
        let body = client.get(url).send().await?.error_for_status()?.text().await?;
        let before = networks.len();
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            networks.push(
                parse_range(line)
                    .ok_or_else(|| anyhow::anyhow!("Invalid range {line:?} from {url}"))?,
            );
        }
        if networks.len() == before {
            anyhow::bail!("{url} listed no ranges");
        }
    }
    Ok(networks)
}

pub async fn init() {
    if *ALLOW_ANY {
        warn!("ALLOW_NON_CLOUDFLARE is set; accepting requests that didn't come through Cloudflare");
    }
    let networks = match fetch().await {
        Ok(networks) => {
            info!("Loaded {} Cloudflare IP ranges", networks.len());
            networks
        }
        Err(e) => {
            warn!("Couldn't fetch Cloudflare IP ranges, so using the built-in list: {e:?}");
            builtin()
        }
    };
    let _ = NETWORKS.set(networks);
}

fn is_cloudflare(ip: IpAddr) -> bool {
    NETWORKS.get_or_init(builtin).iter().any(|&(net, len)| match (ip.to_canonical(), net) {
        (IpAddr::V4(ip), IpAddr::V4(net)) => {
            let mask = u32::MAX.checked_shl(32 - len).unwrap_or(0);
            u32::from(ip) & mask == u32::from(net) & mask
        }
        (IpAddr::V6(ip), IpAddr::V6(net)) => {
            let mask = u128::MAX.checked_shl(128 - len).unwrap_or(0);
            u128::from(ip) & mask == u128::from(net) & mask
        }
        _ => false,
    })
}

fn peer_is_cloudflare(req: &HttpRequest) -> bool {
    req.peer_addr().is_some_and(|a| is_cloudflare(a.ip()))
}

pub fn client_ip(req: &HttpRequest) -> String {
    let forwarded = peer_is_cloudflare(req)
        .then(|| req.headers().get("CF-Connecting-IP")?.to_str().ok())
        .flatten();
    match forwarded {
        Some(ip) => ip.to_string(),
        None => req
            .peer_addr()
            .map(|a| a.ip().to_canonical().to_string())
            .unwrap_or_else(|| "unknown".into()),
    }
}

static BASE_URL: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("BASE_URL")
        .ok()
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
});

pub fn origin(req: &HttpRequest) -> String {
    if let Some(base_url) = BASE_URL.as_ref() {
        return base_url.clone();
    }
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .or_else(|| req.uri().authority().map(|a| a.as_str()))
        .unwrap_or("localhost");
    let scheme = if peer_is_cloudflare(req) {
        req.headers()
            .get("X-Forwarded-Proto")
            .and_then(|h| h.to_str().ok())
            .filter(|p| *p == "http")
            .unwrap_or("https")
    } else {
        "http"
    };
    format!("{scheme}://{host}")
}

pub async fn only_cloudflare<B: MessageBody>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, Error> {
    if *ALLOW_ANY || peer_is_cloudflare(req.request()) {
        return next.call(req).await.map(ServiceResponse::map_into_left_body);
    }
    Ok(req
        .into_response(HttpResponse::Forbidden().finish())
        .map_into_right_body())
}
