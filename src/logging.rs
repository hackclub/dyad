use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};
use chrono_tz::America::New_York;
use log::{Level, Log, Metadata, Record};
use pretty_env_logger::env_logger::{Env, fmt::Color};
use sqlx::types::chrono::Utc;

const MAX_PATH_LEN: usize = 128;

static NEXT_REQ_ID: AtomicU64 = AtomicU64::new(1);
static MAX_TARGET_WIDTH: AtomicUsize = AtomicUsize::new(0);

tokio::task_local! {
    static CONN_CTX: Arc<String>;
}

struct ContextLogger {
    inner: Box<dyn Log>,
}

impl Log for ContextLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if !self.inner.enabled(record.metadata()) {
            return;
        }
        match CONN_CTX.try_get() {
            Ok(tag) => self.inner.log(
                &Record::builder()
                    .metadata(record.metadata().clone())
                    .module_path(record.module_path())
                    .file(record.file())
                    .line(record.line())
                    .args(format_args!("{} [{}]", record.args(), tag))
                    .build(),
            ),
            Err(_) => self.inner.log(record),
        }
    }

    fn flush(&self) {
        self.inner.flush()
    }
}

pub fn init() {
    let logger = pretty_env_logger::formatted_timed_builder()
        .format(|f, record| {
            let time = Utc::now()
                .with_timezone(&New_York)
                .format("%Y-%m-%d %H:%M:%S%.3f %Z");

            let mut level_style = f.style();
            level_style.set_color(match record.level() {
                Level::Trace => Color::Magenta,
                Level::Debug => Color::Blue,
                Level::Info => Color::Green,
                Level::Warn => Color::Yellow,
                Level::Error => Color::Red,
            });

            let target = record.target();
            let width = MAX_TARGET_WIDTH
                .fetch_max(target.len(), Ordering::Relaxed)
                .max(target.len());
            let mut target_style = f.style();
            target_style.set_bold(true);

            writeln!(
                f,
                " {} {} {} > {}",
                time,
                level_style.value(format!("{:<5}", record.level())),
                target_style.value(format!("{target:<width$}")),
                record.args()
            )
        })
        .parse_env(Env::default().default_filter_or("info,sqlx::query=warn"))
        .build();
    let max_level = logger.filter();

    log::set_boxed_logger(Box::new(ContextLogger {
        inner: Box::new(logger),
    }))
    .expect("logger already initialized");
    log::set_max_level(max_level);
}

pub async fn attach_conn_ctx<B: MessageBody>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
    let tag = {
        let ip = crate::cloudflare::client_ip(req.request());
        let path = req.path();
        let path = &path[..path.len().min(MAX_PATH_LEN)];
        format!(
            "#{} {} {} {}",
            NEXT_REQ_ID.fetch_add(1, Ordering::Relaxed),
            ip,
            req.method(),
            path
        )
    };

    CONN_CTX.scope(Arc::new(tag), next.call(req)).await
}
