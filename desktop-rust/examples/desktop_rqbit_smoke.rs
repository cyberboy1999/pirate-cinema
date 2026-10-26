//! Local-only librqbit -> FFmpeg HLS integration check using an owned video fixture.
#[cfg(feature = "desktop")]
fn main() -> Result<(), String> {
    use pirate_cinema_core::{desktop_engine as engine, history::HistoryStore, web_stream};
    use std::{io::Read, path::PathBuf, time::Duration};

    let root = std::env::var_os("PIRATE_CINEMA_DATA_DIR")
        .map(PathBuf::from)
        .ok_or("Set an isolated PIRATE_CINEMA_DATA_DIR")?;
    let mut args = std::env::args().skip(1);
    let fixture = PathBuf::from(args.next().ok_or("torrent or fixture directory required")?);
    let (torrent, videos) = if fixture.is_dir() {
        std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        let created = {
            let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
            runtime.block_on(async {
                let spawner = librqbit::spawn_utils::BlockingSpawner::new(1);
                librqbit::create_torrent(&fixture, Default::default(), &spawner).await
            })
        }
        .map_err(|error| error.to_string())?;
        let torrent = root.join("owned-series.torrent");
        std::fs::write(
            &torrent,
            created.as_bytes().map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let mut videos = std::fs::read_dir(&fixture)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        videos.sort();
        (torrent, videos)
    } else {
        (
            fixture,
            vec![PathBuf::from(
                args.next().ok_or("owned video fixture required")?,
            )],
        )
    };
    if videos.is_empty() {
        return Err("No owned video fixtures".into());
    }
    let bytes = std::fs::read(&torrent).map_err(|error| error.to_string())?;
    let hash = librqbit::torrent_from_bytes(&bytes)
        .map_err(|error| error.to_string())?
        .info_hash
        .as_string();
    // Start the engine first: production startup intentionally clears stale payload.
    engine::read_library("")?;
    let downloads = root.join("rqbit/downloads").join(&hash);
    std::fs::create_dir_all(&downloads).map_err(|error| error.to_string())?;
    for video in &videos {
        let target = downloads.join(video.file_name().ok_or("fixture filename")?);
        std::fs::copy(video, target).map_err(|error| error.to_string())?;
    }
    let imported = engine::import_torrent_file(&torrent, "Local owned fixture")?;
    assert_eq!(imported, hash);
    let files = engine::torrent_video_files("", &hash)?;
    let file = files.first().ok_or("No video in fixture")?;
    let (manifest_url, probe_url, _) = web_stream::prepare(&hash, file)?;
    let address = manifest_url
        .strip_prefix("http://")
        .and_then(|url| url.split('/').next())
        .ok_or("Invalid local HLS URL")?
        .to_owned();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(75)))
        .build()
        .into();

    let probe: serde_json::Value = agent
        .get(&probe_url)
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_json()
        .map_err(|error| error.to_string())?;
    let duration = probe["Duration"].as_f64().ok_or("Missing duration")?;
    assert!(duration > 330.0);

    let manifest = agent
        .get(&manifest_url)
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    assert!(manifest.starts_with("#EXTM3U"));
    assert!(manifest.ends_with("#EXT-X-ENDLIST\n"));
    let (replacement_manifest, _, _) = web_stream::prepare(&hash, file)?;
    agent
        .get(&manifest_url)
        .call()
        .map_err(|error| format!("active manifest retry: {error}"))?;
    agent
        .get(&replacement_manifest)
        .call()
        .map_err(|error| format!("pending replacement lost: {error}"))?;
    let base = replacement_manifest.rsplit_once('/').unwrap().0;
    for segment in ["segment-000000.ts", "segment-000020.ts"] {
        let mut response = agent
            .get(format!("{base}/{segment}"))
            .call()
            .map_err(|error| format!("{segment}: {error}"))?;
        assert_eq!(response.status().as_u16(), 200);
        let mut sample = [0u8; 188];
        response
            .body_mut()
            .as_reader()
            .read_exact(&mut sample)
            .map_err(|error| error.to_string())?;
        assert_eq!(sample[0], 0x47, "{segment} is not MPEG-TS");
    }

    if let Some(next) = files.get(1) {
        let (next_manifest, _, _) = web_stream::prepare(&hash, next)?;
        let manifest = agent
            .get(&next_manifest)
            .call()
            .map_err(|error| format!("next episode manifest: {error}"))?
            .body_mut()
            .read_to_string()
            .map_err(|error| error.to_string())?;
        assert!(manifest.starts_with("#EXTM3U"));
        let next_base = next_manifest.rsplit_once('/').unwrap().0;
        let mut response = agent
            .get(format!("{next_base}/segment-000000.ts"))
            .call()
            .map_err(|error| format!("next episode segment: {error}"))?;
        let mut sample = [0u8; 188];
        response
            .body_mut()
            .as_reader()
            .read_exact(&mut sample)
            .map_err(|error| error.to_string())?;
        assert_eq!(sample[0], 0x47, "next episode is not MPEG-TS");
    }

    let history_path = root.join("history.sqlite");
    let history = HistoryStore::open(&history_path).map_err(|error| error.to_string())?;
    history
        .mark_played(&hash, file.id, &file.name, Some(&file.path))
        .map_err(|error| error.to_string())?;
    history
        .save_progress(&hash, file.id, 63, duration as i64)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        history
            .get(&hash, file.id)
            .map_err(|error| error.to_string())?
            .and_then(|row| row.playback_timecode),
        Some(63)
    );

    let race_url = format!("{base}/segment-000100.ts");
    let race = std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        let _ = agent.get(race_url).call();
    });
    std::thread::sleep(Duration::from_millis(20));
    web_stream::close();
    race.join().map_err(|_| "HLS close race panicked")?;
    assert!(
        std::net::TcpStream::connect(address).is_err(),
        "Owned ArtPlayer HLS server must stop"
    );
    assert!(
        !engine::is_active(&hash)?,
        "Closing ArtPlayer must pause the swarm"
    );
    engine::shutdown();
    println!("PASS: librqbit -> FFmpeg HLS, seek segment, episode switch, history, clean shutdown");
    Ok(())
}

#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("Enable desktop for the ArtPlayer HLS integration check.");
}
