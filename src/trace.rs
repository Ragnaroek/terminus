use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::from_str;

use fundu_core::parse::Parser;
use fundu_core::time::{Multiplier, TimeUnit, TimeUnitsLike};

//{"timestamp":"2024-12-28T17:50:48.993552Z",
//"level":"INFO",
//"fields":{"message":"close","time.busy":"2.93ms","time.idle":"375ns"},
//"target":"iw::time",
//"span":{"name":"calc_tics"},"spans":[{"id":0,"name":"frame"}]}

/*
{"timestamp":"2024-12-28T17:50:49.635111Z",
"level":"INFO","fields":{"message":"close","time.busy":"6.64ms","time.idle":"7.76ms"},
"target":"iw::play",
"span":{"id":3,"name":"frame"},
"spans":[]}
*/

#[derive(Deserialize)]
pub struct Fields {
    pub message: String,

    #[serde(rename = "time.busy")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub time_busy: Duration,

    #[serde(rename = "time.idle")]
    #[serde(deserialize_with = "deserialize_duration")]
    pub time_idle: Duration,
}

#[derive(Deserialize)]
pub struct Span {
    pub id: Option<u64>,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Trace {
    pub target: String,
    pub fields: Fields,
    pub span: Span,
}

pub fn read_trace_file(file: &Path) -> Result<Vec<Trace>, String> {
    let file = File::open(file).map_err(|e| e.to_string())?;
    let lines = BufReader::new(file).lines();

    let mut raw_traces = Vec::new();
    for line in lines.flatten() {
        let trace: Trace = from_str(&line).map_err(|e| e.to_string() + &line)?;
        raw_traces.push(trace);
    }

    let mut result = Vec::new();
    for trace in raw_traces {
        if trace.span.name == "frame" {
            result.push(trace);
        }
    }

    Ok(result)
}

struct TimeUnits {}

impl TimeUnitsLike for TimeUnits {
    #[inline]
    fn is_empty(&self) -> bool {
        false
    }

    fn get(&self, identifier: &str) -> Option<(TimeUnit, Multiplier)> {
        match identifier {
            "ns" => Some((TimeUnit::NanoSecond, Multiplier(1, 0))),
            "µs" => Some((TimeUnit::MicroSecond, Multiplier(1, 0))),
            "ms" => Some((TimeUnit::MilliSecond, Multiplier(1, 0))),
            "s" => Some((TimeUnit::Second, Multiplier(1, 0))),
            "m" => Some((TimeUnit::Minute, Multiplier(1, 0))),
            "h" => Some((TimeUnit::Hour, Multiplier(1, 0))),
            "d" => Some((TimeUnit::Day, Multiplier(1, 0))),
            "w" => Some((TimeUnit::Week, Multiplier(1, 0))),
            _ => None,
        }
    }
}

const DURATION_PARSER: Parser = Parser::new();
const TIME_UNITS: TimeUnits = TimeUnits {};

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let duration = DURATION_PARSER
        .parse(&buf, &TIME_UNITS, None, None)
        .map_err(serde::de::Error::custom)?;
    Ok(duration.try_into().unwrap())
}
