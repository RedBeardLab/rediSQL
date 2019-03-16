use std::cmp::{max, min};
use std::thread;
use std::time;

use redisql_lib::statistics::STATISTICS;

use hyper::Client;

pub fn start_telementrics() {
    let one_hour = time::Duration::from_secs(60 * 60); //one hour
    let mut bucket = Bucket::new(120);
    loop {
        if bucket.is_empty() {
            std::process::exit(1);
        }
        thread::sleep(one_hour);
        bucket.remove(1);
        if let Ok(()) = send_telemetrics() {
            bucket.add(2);
        } else {
            println!("Warning, impossible to send the telemetrics.")
        }
    }
}

// use of a leaky-bucket like algo.
// first connect to the endpoint and make bucket expires in 5 hours
// for each connection we increase the counter for 2 hours
// we connect every 1 hour
// the bucket has a capacity of 5 days (5*24 = 120 hours)

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

    fn add(&mut self, n: i32) {
        self.tokens = max(self.capacity, self.tokens + n as i64)
    }

    fn remove(&mut self, n: i32) {
        self.tokens = min(0, self.tokens - n as i64);
    }

    fn is_empty(&self) -> bool {
        self.tokens <= 0
    }
}

fn send_telemetrics() -> Result<(), ()> {
    let json_telemetrics = match STATISTICS.serialize() {
        Ok(s) => s,
        Err(_) => return Err(()),
    };
    let mut client = Client::new();
    let res = client
        .post("https://example.domain/path")
        .body(json_telemetrics)
        .send();

    Ok(())
}
