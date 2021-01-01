use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    let sleep = async_fuse::fuse(time::sleep(Duration::from_millis(100)));
    tokio::pin!(sleep);

    for _ in 0..20usize {
        (&mut sleep).await;
        assert!(sleep.is_empty());
        sleep.set(async_fuse::fuse(time::sleep(Duration::from_millis(100))))
    }
}
