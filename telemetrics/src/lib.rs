#[macro_use]
extern crate log;

use std::cmp::{max, min};
use std::thread;
use std::time;

use redisql_lib::statistics::STATISTICS;

static PRIMARY_TELEMETRICS_URL: &str =
    "https://telemetrics.redisql.com/v0/statistics";
static SECONDARY_TELEMETRICS_URL: &str =
    "https://telemetrics.redisql.redbeardlab.com/v0/statistics";
static TERTIARY_TELEMETRICS_URL: &str =
    "http://telemetrics.redisql.redbeardlab.com/v0/statistics";

// use of a leaky-bucket like algo.
// first connect to the endpoint and make bucket expires in 5 hours
// for each connection we increase the counter for another 5 hours
// we connect every 1 hour
// the bucket has a capacity of 5 days (5*24 = 120 hours)

pub fn start_telemetrics() {
    let one_hour = time::Duration::from_secs(60 * 60); //one hour
    let mut bucket = Bucket::new(120);
    if send_telemetrics().is_ok() {
        bucket.add(5);
    } else {
        warn!("Warning, impossible to send the telemetrics.")
    }
    loop {
        if bucket.is_empty() {
            error!("Telemetrics not reachables, exit!");
            std::process::exit(1);
        }
        thread::sleep(one_hour);
        bucket.remove(1);
        if send_telemetrics().is_ok() {
            bucket.add(5);
        } else {
            warn!("Warning, impossible to send the telemetrics.")
        }
    }
}

struct Bucket {
    tokens: i64,
    capacity: i64,
}

impl Bucket {
    fn new(capacity: i64) -> Bucket {
        Bucket {
            capacity,
            tokens: 0,
        }
    }

    fn add(&mut self, n: i64) {
        self.tokens = min(self.capacity, self.tokens + n)
    }

    fn remove(&mut self, n: i64) {
        self.tokens = max(0, self.tokens - n);
    }

    fn is_empty(&self) -> bool {
        self.tokens <= 0
    }
}

fn send_telemetrics() -> Result<(), ()> {
    let json_telemetrics = match STATISTICS.serialize() {
        Ok(s) => s,
        Err(_) => {
            warn!("Error in getting the stats!");
            return Err(());
        }
    };
    for url in &[
        PRIMARY_TELEMETRICS_URL,
        SECONDARY_TELEMETRICS_URL,
        TERTIARY_TELEMETRICS_URL,
    ] {
        let res = ureq::post(*url)
            .timeout_connect(5_000)
            .timeout_read(5_000)
            .send_string(&json_telemetrics.clone());
        match res.ok() {
            false => {
                let err = match res.into_string() {
                    Ok(ok) => ok,
                    Err(e) => e.to_string(),
                };
                warn!(
                    "Error in making the request to {}: {}",
                    *url, err
                );
            }
            true => {
                return Ok(());
            }
        }
    }
    Err(())
}
