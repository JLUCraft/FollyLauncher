pub static AUTHLIB_INJECTOR_JAR_NAME: &str = "authlib-injector.jar";
/// The only built-in authentication server. It publishes `shared_client_id` in its OpenID
/// configuration, so it needs no entry in `CLIENT_IDS`.
pub static PRESET_AUTH_SERVERS: [&str; 1] = ["https://skin.jlucraft.com"];
pub static SCOPE: &str =
  "openid offline_access Yggdrasil.PlayerProfiles.Select Yggdrasil.Server.Join";

/// Client ids of MUA federation members a user may add by hand
/// (ref: https://github.com/SJMC-Dev/SJMCL-client-ids).
pub static CLIENT_IDS: [(&str, &str); 6] = [
  ("skin.mc.sjtu.cn", "6"),
  ("skin.mualliance.ltd", "27"),
  ("hall.mc.taru.xj.cn", "yggc_kPHIuWBoMhlkThX1"),
  ("oauth.ustb.world", "6"),
  ("skin.jsumc.fun", "2"),
  ("user.suesmc.ltd", "4"),
];
