#![allow(dead_code)]
use crate::netem::Distribution;
use crate::{Error, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct NetemConfig {
    pub limit: Option<i32>,
    pub interface: String,
    pub network_latency: f64,
    pub jitter: Option<f64>,
    pub correlation: Option<f64>,
    pub distribution: Option<Distribution>,
}

impl NetemConfig {
    pub fn build() -> Result<Self, Error> {
        let pod_name = env::var("NODE_NAME").map(|node_name| node_name.to_uppercase())?;
        let network_latency = env::var(pod_name).map(|latency| latency.parse::<f64>())??;

        let limit = env::var("LIMIT")
            .map(|limit| limit.parse::<i32>())
            .ok()
            .transpose()?;

        let interface = env::var("INTERFACE").map_err(Error::LatencyConfigEnvError)?;

        let jitter = env::var("JITTER")
            .map(|jitter| jitter.parse::<f64>())
            .ok()
            .transpose()?;

        let correlation = env::var("CORRELATION")
            .map(|correlation| correlation.parse::<f64>())
            .ok()
            .transpose()?;

        let distribution = env::var("DISTRIBUTION")
            .map(|dist| {
                dist.parse::<Distribution>().map_err(|_| {
                    Error::LatencyConfigParseError("invalid distribution value".to_owned())
                })
            })
            .ok()
            .transpose()?;

        let netem_config = Self {
            limit,
            interface,
            network_latency,
            jitter,
            correlation,
            distribution,
        };

        Ok(netem_config)
    }
}
