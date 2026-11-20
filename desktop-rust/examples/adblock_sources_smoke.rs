fn main() -> Result<(), String> {
    let root = pirate_cinema_core::desktop_engine::profile_dir()?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let path = root.join("settings.json");
    pirate_cinema_core::settings::save_adblock(&path, false)?;
    assert!(!pirate_cinema_core::adblock::blocked(
        "https://rutube.ru/api/advert/vast/a"
    ));
    pirate_cinema_core::settings::save_adblock(&path, true)?;
    assert!(pirate_cinema_core::adblock::blocked(
        "https://rutube.ru/api/advert/vast/a"
    ));
    println!("{}", pirate_cinema_core::adblock::update()?);
    assert!(root.join("adblock-network.json").is_file());
    pirate_cinema_core::settings::save_adblock(&path, false)?;
    assert!(!pirate_cinema_core::adblock::blocked(
        "https://rutube.ru/api/advert/vast/a"
    ));
    println!("PASS: real source download, cache persistence, enabled and disabled requests");
    Ok(())
}
