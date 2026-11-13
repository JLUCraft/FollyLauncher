use crate::brand_meta::DEEPLINK_PREFIX;
use rmcp::handler::server::tool::ToolRoute;

use crate::intelligence::mcp_server::launcher::McpContext;
use crate::mcp_tool;

pub fn tool_routes() -> Vec<ToolRoute<McpContext>> {
  vec![mcp_tool!(
    deeplink "launch_instance",
    "Launch a specific Minecraft instance by its instance_id via launcher deeplink. Before launch, check and update, if user requested, the selected player in launcher configuration.",
    |params|
    #[serde(deny_unknown_fields)]
    {
      #[schemars(description = "Minecraft instance ID.")]
      instance_id: String,
    } => format!(
      "{DEEPLINK_PREFIX}launch?id={}",
      urlencoding::encode(&params.instance_id)
    )
  )]
}
