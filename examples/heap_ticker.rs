use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    let mut duration = Duration::from_millis(500);

    let mut sleep = async_fuse::Fuse::pin(time::sleep(duration));
    let mut update_duration = async_fuse::Fuse::pin(time::sleep(Duration::from_secs(2)));

    for _ in 0..10usize {
        tokio::select! {
            _ = &mut sleep => {
                println!("Tick");
                sleep.set(Box::pin(time::sleep(duration)))
            }
            _ = &mut update_duration => {
                println!("Tick faster!");
                duration = Duration::from_millis(250);
            }
        }
    }
}
