#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::endpoints::run_http_server;
use crate::logger::{init_logger, log_error};
use crate::tables::MimeInfoShared;
use actix_web::web::Data as ActixData;
use futures::join as wait_for_all;
use tokio::time::{sleep_until, Duration, Instant};

pub(crate) mod endpoints;
pub(crate) mod logger;
pub(crate) mod tables;

const MIME_UPDATE_INTERVAL: Duration = Duration::from_secs(5 * 60);

async fn mime_updater(shared_state: ActixData<MimeInfoShared>) {
    let mut next_deadline = Instant::now() + MIME_UPDATE_INTERVAL;

    loop {
        sleep_until(next_deadline).await;

        if let Err(err) = shared_state.update_all().await {
            log_error!("{err}");
        }

        next_deadline += MIME_UPDATE_INTERVAL;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    let shared_state: ActixData<MimeInfoShared> = MimeInfoShared::default().into();
    let mime_updater_task = mime_updater(shared_state.clone());
    let http_server_task = run_http_server(shared_state);

    let _ = wait_for_all!(mime_updater_task, http_server_task,);

    Ok(())
}
