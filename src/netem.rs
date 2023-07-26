use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::process::Command;

type Percentage = f64;
type Millisecond = f64;

trait ToPercentageString {
    fn to_pct_string(&self) -> String;
}

impl ToPercentageString for Percentage {
    fn to_pct_string(&self) -> String {
        format!("{:.02}%", self)
    }
}

trait ToMillisecondString {
    fn to_ms_string(&self) -> String;
}

impl ToMillisecondString for Millisecond {
    fn to_ms_string(&self) -> String {
        format!("{}ms", self)
    }
}

trait Control {
    fn to_args(&self) -> Vec<String>;
}

// LIMIT := limit packets
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Limit {
    packets: i32,
}

impl Control for Limit {
    fn to_args(&self) -> Vec<String> {
        vec!["limit".into(), format!("{}", self.packets)]
    }
}

static LIMIT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"limit\s(?P<packets>[-\d]+)").expect("Failed to create regex of limit")
});

impl FromStr for Limit {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(captures) = LIMIT_REGEX.captures(s) {
            let packets: i32 = captures
                .name("packets")
                .ok_or_else(|| anyhow::anyhow!("Failed to get limit packets from '{}'", s))?
                .as_str()
                .parse()?;

            Ok(Limit { packets })
        } else {
            Err(anyhow::anyhow!("no limit"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
enum Distribution {
    #[serde(rename = "uniform")]
    Uniform,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "pareto")]
    Pareto,
    #[serde(rename = "paretonormal")]
    ParetoNormal,
}

impl From<Distribution> for String {
    fn from(distribution: Distribution) -> Self {
        match distribution {
            Distribution::Uniform => "uniform",
            Distribution::Normal => "normal",
            Distribution::Pareto => "pareto",
            Distribution::ParetoNormal => "paretonormal",
        }
        .to_string()
    }
}

/// DELAY := delay TIME [ JITTER [ CORRELATION ]]]
///        [ distribution { uniform | normal | pareto |  paretonormal } ]
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct Delay {
    time: Millisecond,
    #[serde(skip_serializing_if = "Option::is_none")]
    jitter: Option<Millisecond>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation: Option<Percentage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distribution: Option<Distribution>,
}

impl Control for Delay {
    fn to_args(&self) -> Vec<String> {
        let mut v = Vec::with_capacity(3);

        v.push("delay".into());
        v.push(self.time.to_ms_string());

        if let Some(jitter) = self.jitter {
            v.push(jitter.to_ms_string());
            if let Some(correlation) = self.correlation {
                v.push(correlation.to_pct_string());
            }
        }

        if let Some(distribution) = self.distribution {
            v.push("distribution".into());
            v.push(distribution.into());
        }

        v
    }
}

static DELAY_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"delay\s(?P<time>[\d\.]+)ms(\s{2}(?P<jitter>[\d\.]+)ms\s((?P<correlation>[\d\.]+)%)?)?",
    )
    .expect("Failed to create regex of delay")
});

impl FromStr for Delay {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(captures) = DELAY_REGEX.captures(s) {
            let time: Millisecond = captures
                .name("time")
                .ok_or_else(|| anyhow::anyhow!("Failed to get delay time from '{}'", s))?
                .as_str()
                .parse()?;

            let jitter: Option<Millisecond> = match captures.name("jitter") {
                Some(s) => s.as_str().parse().ok(),
                None => None,
            };

            let correlation: Option<Percentage> = if jitter.is_some() {
                match captures.name("correlation") {
                    Some(s) => s.as_str().parse().ok(),
                    None => None,
                }
            } else {
                None
            };

            Ok(Delay {
                time,
                jitter,
                correlation,
                distribution: None,
            })
        } else {
            Err(anyhow::anyhow!("no delay"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Controls {
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<Limit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay: Option<Delay>,
}

impl Control for Controls {
    fn to_args(&self) -> Vec<String> {
        let mut v = Vec::new();

        if let Some(limit) = &self.limit {
            v.append(&mut limit.to_args());
        }

        if let Some(delay) = &self.delay {
            v.append(&mut delay.to_args());
        }

        v
    }
}

impl FromStr for Controls {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("qdisc netem") {
            return Ok(Controls::default());
        }

        let limit = Limit::from_str(s).ok();
        let delay = Delay::from_str(s).ok();

        Ok(Controls { limit, delay })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum NetEm {
    #[serde(rename = "set")]
    Set {
        interface: String,
        controls: Controls,
    },
    #[serde(rename = "reset")]
    Reset { interface: String },
}

impl NetEm {
    async fn do_execute(&self) -> anyhow::Result<Output> {
        let args = self.to_args();
        log::info!("Executing => tc {}", args.join(" "));
        let output = Command::new("tc")
            .args(args)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Command Error: {}", e))?;
        let output = if let Some(code) = output.status.code() {
            if code == 0 {
                Output::Ok
            } else {
                let description = match String::from_utf8(output.stderr) {
                    Ok(stderr) => {
                        format!("Exit with status code: {}, stderr: {}", code, stderr)
                    }
                    Err(_) => format!("Exit with status code: {}", code),
                };
                Output::err(description)
            }
        } else {
            Output::err("Process killed by signal".to_owned())
        };

        Ok(output)
    }
    pub async fn execute(&self) -> Output {
        match self.do_execute().await {
            Ok(output) => output,
            Err(e) => Output::err(e.to_string()),
        }
    }
}

impl Control for NetEm {
    fn to_args(&self) -> Vec<String> {
        match self {
            NetEm::Set {
                interface,
                controls,
            } => {
                // tc qdisc replace dev <INTERFACE> root netem delay 100ms 10ms loss 1% 30% duplicate 1% reorder 10% 50% corrupt 0.2%
                let mut args = vec![
                    "qdisc".into(),
                    "replace".into(),
                    "dev".into(),
                    interface.into(),
                    "root".into(),
                    "netem".into(),
                ];

                args.append(&mut controls.to_args());

                args
            }
            NetEm::Reset { interface } => {
                // tc qdisc del dev <INTERFACE> root netem
                vec![
                    "qdisc".into(),
                    "del".into(),
                    "dev".into(),
                    interface.into(),
                    "root".into(),
                    "netem".into(),
                ]
            }
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "status")]
pub enum Output {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error { description: String },
}

impl Output {
    pub fn err(description: String) -> Self {
        Output::Error { description }
    }
}
