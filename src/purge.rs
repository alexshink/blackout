pub fn wipe() -> Result<(), String> {
    crate::autostart::remove_if_ours()?;
    crate::config::remove_our_files()?;
    Ok(())
}

pub fn dialog_body(lang: crate::i18n::Language) -> String {
    let t = lang.tr();
    let config = match crate::config::strict_config_path() {
        Ok(path) => t
            .wipe_config_fmt
            .replace("{path}", &path.display().to_string()),
        Err(_) => t.wipe_config_unavailable.to_string(),
    };
    t.wipe_body.replace("{config}", &config)
}
