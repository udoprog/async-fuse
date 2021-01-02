use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    let mut duration = Duration::from_millis(500);

    let sleep = async_fuse::Fuse::new(time::sleep(duration));
    tokio::pin!(sleep);

    let update_duration = async_fuse::Fuse::new(time::sleep(Duration::from_secs(2)));
    tokio::pin!(update_duration);

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
