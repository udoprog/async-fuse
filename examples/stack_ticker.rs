use std::pin::pin;
use std::time::Duration;

use async_fuse::Fuse;
use tokio::time;

#[tokio::main]
async fn main() {
    let mut duration = Duration::from_millis(500);

    let mut sleep = pin!(Fuse::new(time::sleep(duration)));
    let mut update_duration = pin!(Fuse::new(time::sleep(Duration::from_secs(2))));

    for _ in 0..10usize {
        tokio::select! {
            _ = &mut sleep => {
                println!("Tick");
                sleep.set(async_fuse::Fuse::new(time::sleep(duration)));
            }
            _ = &mut update_duration => {
                println!("Tick faster!");
                duration = Duration::from_millis(250);
            }
        }
    }
}
