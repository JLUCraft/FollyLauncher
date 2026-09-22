use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf};
use terminal_runtime::{NodeArgs, TargetArgs, init_logging, load_grant, run_proxy};
use union_core::ServiceId;

#[derive(Parser)]
#[command(version, about = "Federated player launcher foundation")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List installed Minecraft instances using the existing minecraftd library.
    LocalInstances,
    /// Expose a local TCP endpoint for an existing Minecraft client.
    Proxy {
        #[command(flatten)]
        node: Box<NodeArgs>,
        #[command(flatten)]
        target: TargetArgs,
        #[arg(long)]
        service: ServiceId,
        #[arg(long)]
        ticket: Option<PathBuf>,
        #[arg(long, default_value = "127.0.0.1:25566")]
        bind: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    match Args::parse().command {
        Command::LocalInstances => {
            for instance in minecraftd::discovery::discover() {
                println!(
                    "{}\t{}\t{}",
                    instance.id,
                    instance.kind.as_str(),
                    instance.path.display()
                );
            }
        }
        Command::Proxy {
            node,
            target,
            service,
            ticket,
            bind,
        } => {
            let grant = load_grant(ticket.as_deref())?;
            run_proxy(node.start(false).await?, target, service, grant, bind).await?;
        }
    }
    Ok(())
}
