use tauri::AppHandle;
pub fn get_minecraft_lang_tag(
  locale: &str,
  version: &str,
  app: &AppHandle,
) -> Option<&'static str> {
  minecraftd::minecraft::options::get_minecraft_lang_tag(
    locale,
    version,
    crate::instance::helpers::game_version::build_game_version_cmp_fn(app),
  )
}
