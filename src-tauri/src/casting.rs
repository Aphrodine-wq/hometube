use anyhow::{Context, Result};
use oxicast::{
    CastApp, CastClient, IdleReason, MediaInfo, MediaMetadata, MediaStatus, PlayerState,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const REACHABILITY_TIMEOUT: Duration = Duration::from_millis(900);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(20);
const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastDeviceInfo {
    pub id: String,
    pub name: String,
    pub model: Option<String>,
    pub ip: String,
    pub port: u16,
    pub available: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CastStartRequest {
    pub device: CastDeviceInfo,
    pub media_id: String,
    pub position_secs: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastStatus {
    pub connected: bool,
    pub device_name: Option<String>,
    pub media_id: Option<String>,
    pub state: String,
    pub current_time: f64,
    pub duration: f64,
    pub volume: f32,
    pub muted: bool,
    pub idle_reason: Option<String>,
}

impl CastStatus {
    fn disconnected() -> Self {
        Self {
            connected: false,
            device_name: None,
            media_id: None,
            state: "disconnected".into(),
            current_time: 0.0,
            duration: 0.0,
            volume: 0.0,
            muted: false,
            idle_reason: None,
        }
    }
}

struct CastSession {
    client: CastClient,
    device_name: String,
    media_id: String,
    last_media: Option<MediaStatus>,
    last_media_at: Instant,
}

#[derive(Default)]
pub struct CastRuntime {
    session: Mutex<Option<CastSession>>,
}

impl CastRuntime {
    pub fn active_media_id(&self) -> Option<String> {
        self.session
            .lock()
            .as_ref()
            .map(|session| session.media_id.clone())
    }

    pub async fn discover(&self) -> Result<Vec<CastDeviceInfo>> {
        let mut devices = oxicast::discovery::discover_devices(Duration::from_secs(3))
            .await
            .context("Could not scan the local network for Cast devices")?
            .into_iter()
            .map(|device| {
                let mut info = CastDeviceInfo {
                    id: device.uuid.unwrap_or_else(|| device.ip.to_string()),
                    name: device.name,
                    model: device.model,
                    ip: device.ip.to_string(),
                    port: device.port,
                    available: false,
                };
                info.available = device_reachable(&info);
                eprintln!(
                    "[hometube:cast] discovered name={} address={}:{} available={}",
                    info.name, info.ip, info.port, info.available
                );
                info
            })
            .collect::<Vec<_>>();
        devices.sort_by_key(|device| device.name.to_lowercase());
        Ok(devices)
    }

    pub async fn start(
        &self,
        request: &CastStartRequest,
        media_url: &str,
        title: &str,
        creator: &str,
        duration: f64,
    ) -> Result<CastStatus> {
        validate_device(&request.device)?;
        ensure_device_reachable(&request.device)?;
        let previous = { self.session.lock().take() };
        if let Some(previous) = previous {
            let _ = previous.client.disconnect().await;
        }

        eprintln!(
            "[hometube:cast] stage=connect name={} address={}:{}",
            request.device.name, request.device.ip, request.device.port
        );
        let client = tokio::time::timeout(
            CONNECT_TIMEOUT,
            CastClient::builder(&request.device.ip, request.device.port)
                .request_timeout(Duration::from_secs(10))
                .connect(),
        )
        .await
        .with_context(|| {
            format!(
                "{} did not accept a Cast connection within {} seconds",
                request.device.name,
                CONNECT_TIMEOUT.as_secs()
            )
        })?
        .with_context(|| format!("Could not connect to {}", request.device.name))?;
        eprintln!("[hometube:cast] stage=connect result=ok");

        eprintln!("[hometube:cast] stage=launch_receiver");
        tokio::time::timeout(
            LAUNCH_TIMEOUT,
            client.launch_app(&CastApp::DefaultMediaReceiver),
        )
        .await
        .with_context(|| {
            format!(
                "{} did not open the Cast receiver within {} seconds",
                request.device.name,
                LAUNCH_TIMEOUT.as_secs()
            )
        })?
        .with_context(|| {
            format!(
                "{} could not open the Cast media receiver",
                request.device.name
            )
        })?;
        eprintln!("[hometube:cast] stage=launch_receiver result=ok");
        let media = MediaInfo::new(media_url, "video/mp4")
            .duration(duration)
            .metadata(MediaMetadata::Generic {
                title: Some(title.into()),
                subtitle: Some(creator.into()),
                images: Vec::new(),
            });
        eprintln!("[hometube:cast] stage=load_media");
        let media_status = tokio::time::timeout(
            LOAD_TIMEOUT,
            client.load_media(&media, true, request.position_secs.max(0.0), None),
        )
        .await
        .with_context(|| {
            format!(
                "{} could not fetch the HomeTube video within {} seconds. Check that the TV and this computer are on the same Wi-Fi network",
                request.device.name,
                LOAD_TIMEOUT.as_secs()
            )
        })?
            .with_context(|| format!("{} could not load this video", request.device.name))?;
        eprintln!("[hometube:cast] stage=load_media result=ok");
        let session = CastSession {
            client,
            device_name: request.device.name.clone(),
            media_id: request.media_id.clone(),
            last_media: Some(media_status.clone()),
            last_media_at: Instant::now(),
        };
        let status = status_from_media(&session, Some(media_status), Duration::ZERO);
        *self.session.lock() = Some(session);
        Ok(status)
    }

    pub async fn load_next(
        &self,
        media_id: &str,
        media_url: &str,
        title: &str,
        creator: &str,
        duration: f64,
        position_secs: f64,
    ) -> Result<CastStatus> {
        let snapshot = self.client_snapshot()?;
        if !snapshot.client.is_connected() {
            anyhow::bail!("The Cast connection closed");
        }
        let media = MediaInfo::new(media_url, "video/mp4")
            .duration(duration)
            .metadata(MediaMetadata::Generic {
                title: Some(title.into()),
                subtitle: Some(creator.into()),
                images: Vec::new(),
            });
        let media_status = tokio::time::timeout(
            LOAD_TIMEOUT,
            snapshot
                .client
                .load_media(&media, true, position_secs.max(0.0), None),
        )
        .await
        .context("The Cast device stopped responding while loading the next video")??;
        let mut guard = self.session.lock();
        let session = guard.as_mut().context("No Cast device is connected")?;
        session.media_id = media_id.into();
        session.last_media = Some(media_status.clone());
        session.last_media_at = Instant::now();
        Ok(status_from_media(
            session,
            Some(media_status),
            Duration::ZERO,
        ))
    }

    pub async fn status(&self) -> Result<CastStatus> {
        let mut session_guard = self.session.lock();
        let Some(session) = session_guard.as_mut() else {
            return Ok(CastStatus::disconnected());
        };
        if !session.client.is_connected() {
            anyhow::bail!("The Cast connection closed");
        }

        let watcher = session.client.watch_media_status();
        let latest = watcher.borrow().clone();
        if latest.is_some() && latest != session.last_media {
            session.last_media = latest;
            session.last_media_at = Instant::now();
        }
        let media = session.last_media.clone();
        let age = session.last_media_at.elapsed();
        let mut status = status_from_media(session, media, age);
        let receiver_watcher = session.client.watch_receiver_status();
        if let Some(receiver) = receiver_watcher.borrow().as_ref() {
            status.volume = receiver.volume.level;
            status.muted = receiver.volume.muted;
        }
        Ok(status)
    }

    pub async fn set_playing(&self, playing: bool) -> Result<CastStatus> {
        let session = self.client_snapshot()?;
        let command = async {
            if playing {
                session.client.play().await
            } else {
                session.client.pause().await
            }
        };
        let status = tokio::time::timeout(COMMAND_TIMEOUT, command)
            .await
            .context("The Cast device stopped responding")??;
        Ok(status_from_media(&session, Some(status), Duration::ZERO))
    }

    pub async fn seek(&self, position_secs: f64) -> Result<CastStatus> {
        let session = self.client_snapshot()?;
        let status =
            tokio::time::timeout(COMMAND_TIMEOUT, session.client.seek(position_secs.max(0.0)))
                .await
                .context("The Cast device stopped responding")??;
        Ok(status_from_media(&session, Some(status), Duration::ZERO))
    }

    pub async fn set_volume(&self, volume: f32) -> Result<CastStatus> {
        let session = self.client_snapshot()?;
        tokio::time::timeout(
            COMMAND_TIMEOUT,
            session.client.set_volume(volume.clamp(0.0, 1.0)),
        )
        .await
        .context("The Cast device stopped responding")??;
        self.status().await
    }

    pub async fn set_muted(&self, muted: bool) -> Result<CastStatus> {
        let session = self.client_snapshot()?;
        tokio::time::timeout(COMMAND_TIMEOUT, session.client.set_muted(muted))
            .await
            .context("The Cast device stopped responding")??;
        self.status().await
    }

    pub async fn stop(&self) -> Result<CastStatus> {
        let session = { self.session.lock().take() };
        if let Some(session) = session {
            session.client.disconnect().await?;
        }
        Ok(CastStatus::disconnected())
    }

    fn client_snapshot(&self) -> Result<CastSession> {
        self.session
            .lock()
            .as_ref()
            .map(|session| CastSession {
                client: session.client.clone(),
                device_name: session.device_name.clone(),
                media_id: session.media_id.clone(),
                last_media: session.last_media.clone(),
                last_media_at: session.last_media_at,
            })
            .context("No Cast device is connected")
    }
}

fn status_from_media(
    session: &CastSession,
    media: Option<MediaStatus>,
    status_age: Duration,
) -> CastStatus {
    let Some(media) = media else {
        return CastStatus {
            connected: true,
            device_name: Some(session.device_name.clone()),
            media_id: Some(session.media_id.clone()),
            state: "connected".into(),
            current_time: 0.0,
            duration: 0.0,
            volume: 0.0,
            muted: false,
            idle_reason: None,
        };
    };
    let state = match media.player_state {
        PlayerState::Playing => "playing",
        PlayerState::Paused => "paused",
        PlayerState::Buffering => "buffering",
        PlayerState::Idle => "idle",
        _ => "connected",
    };
    let current_time = if media.player_state == PlayerState::Playing {
        media.current_time + status_age.as_secs_f64()
    } else {
        media.current_time
    };
    let duration = media.duration.unwrap_or_default();
    CastStatus {
        connected: true,
        device_name: Some(session.device_name.clone()),
        media_id: Some(session.media_id.clone()),
        state: state.into(),
        current_time: if duration > 0.0 {
            current_time.min(duration)
        } else {
            current_time
        },
        duration,
        volume: media.volume.level,
        muted: media.volume.muted,
        idle_reason: media.idle_reason.map(|reason| {
            match reason {
                IdleReason::Finished => "finished",
                IdleReason::Cancelled => "cancelled",
                IdleReason::Interrupted => "interrupted",
                IdleReason::Error => "error",
                _ => "unknown",
            }
            .into()
        }),
    }
}

pub(crate) fn validate_device(device: &CastDeviceInfo) -> Result<()> {
    let ip: IpAddr = device
        .ip
        .parse()
        .context("Cast device returned an invalid address")?;
    let local = match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_link_local() || ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_unique_local() || ip.is_unicast_link_local() || ip.is_loopback(),
    };
    if !local || device.port == 0 {
        anyhow::bail!("Cast devices must be on your local network");
    }
    Ok(())
}

fn socket_address(device: &CastDeviceInfo) -> Result<SocketAddr> {
    let ip = device
        .ip
        .parse::<IpAddr>()
        .context("Cast device returned an invalid address")?;
    Ok(SocketAddr::new(ip, device.port))
}

fn device_reachable(device: &CastDeviceInfo) -> bool {
    socket_address(device)
        .and_then(|address| {
            TcpStream::connect_timeout(&address, REACHABILITY_TIMEOUT).map_err(Into::into)
        })
        .is_ok()
}

fn ensure_device_reachable(device: &CastDeviceInfo) -> Result<()> {
    let address = socket_address(device)?;
    TcpStream::connect_timeout(&address, REACHABILITY_TIMEOUT).with_context(|| {
        format!(
            "{} is offline or asleep. Turn on the TV, confirm it is on the same Wi-Fi network, then Scan again",
            device.name
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_local_cast_addresses() {
        let mut device = CastDeviceInfo {
            id: "tv".into(),
            name: "Living Room".into(),
            model: None,
            ip: "192.168.1.20".into(),
            port: 8009,
            available: true,
        };
        assert!(validate_device(&device).is_ok());
        device.ip = "8.8.8.8".into();
        assert!(validate_device(&device).is_err());
    }
}
