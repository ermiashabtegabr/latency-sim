use std::str::FromStr;
use tokio::process::Command;
use tracing::info;

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    packets: i32,
}

impl Limit {
    pub fn new(packets: i32) -> Self {
        Self { packets }
    }
}

impl Control for Limit {
    fn to_args(&self) -> Vec<String> {
        vec!["limit".into(), format!("{}", self.packets)]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Distribution {
    Uniform,
    Normal,
    Pareto,
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

impl FromStr for Distribution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dis = match s {
            "uniform" => Ok(Distribution::Uniform),
            "normal" => Ok(Distribution::Normal),
            "pareto" => Ok(Distribution::Pareto),
            "paretonormal" => Ok(Distribution::ParetoNormal),
            _ => Err(anyhow::anyhow!("no distribution")),
        };

        dis
    }
}

/// DELAY := delay TIME [ JITTER [ CORRELATION ]]]
///        [ distribution { uniform | normal | pareto |  paretonormal } ]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Delay {
    pub time: Millisecond,
    pub jitter: Option<Millisecond>,
    pub correlation: Option<Percentage>,
    pub distribution: Option<Distribution>,
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

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Controls {
    pub limit: Option<Limit>,
    pub delay: Option<Delay>,
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

#[derive(Clone, Debug)]
pub struct NetEm {
    pub interface: String,
    pub controls: Controls,
}

impl NetEm {
    async fn do_execute(&self) -> anyhow::Result<Output> {
        let args = self.to_args();
        info!("Executing => tc {}", args.join(" "));
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
                    Ok(stderr) => format!("Exit with status code: {}, stderr: {}", code, stderr),
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
        let interface = self.interface.clone();
        let mut args = vec![
            "qdisc".into(),
            "replace".into(),
            "dev".into(),
            interface.into(),
            "root".into(),
            "netem".into(),
        ];
        args.append(&mut self.controls.to_args());
        args
    }
}

#[derive(Clone, Debug)]
pub enum Output {
    Ok,
    Error { description: String },
}

impl Output {
    pub fn err(description: String) -> Self {
        Output::Error { description }
    }
}
