use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    let mut interval = async_fuse::poll_fn(
        time::interval(Duration::from_secs(1)),
        time::Interval::poll_tick,
    );

    let sleep = async_fuse::once(time::sleep(Duration::from_secs(5)));
    tokio::pin!(sleep);

    for _ in 0..20usize {
        tokio::select! {
            when = &mut interval => {
                println!("tick: {:?}", when);
            }
            _ = &mut sleep => {
                interval.set(time::interval(Duration::from_millis(200)));
            }
        }
    }
}
