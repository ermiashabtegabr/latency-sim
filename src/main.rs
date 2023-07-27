use latency_sim::{config::NetemConfig, Controls, Delay, Limit, NetEm, Output};
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let netem_config = NetemConfig::build()?;
    let limit = netem_config.limit.map(|limit| Limit::new(limit));
    let delay = Some(Delay {
        time: netem_config.network_latency,
        jitter: netem_config.jitter,
        correlation: netem_config.correlation,
        distribution: None,
    });

    let controls = Controls { limit, delay };
    let interface = netem_config.interface.clone();
    let netem = NetEm {
        interface,
        controls,
    };

    match netem.execute().await {
        Output::Ok => info!("latency applied"),
        Output::Error { description } => error!("failed to apply latecy: {}", description),
    }

    Ok(())
}
