use clap::Clap;
use futures::FutureExt;
use hyper::{
    body,
    http::Result as HyperResult,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::{error, info};
use simple_logger::SimpleLogger;
use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    process,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::fs;

#[derive(Clap, Clone)]
struct Config {
    #[clap(long)]
    wiki_file: String,
    #[clap(long)]
    backup_dir: String,
    #[clap(long, default_value = "0.0.0.0")]
    address: String,
    #[clap(long)]
    port: u16,
}

fn gen_backup_filename(backup_dir: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    Path::new(backup_dir).join(&format!("backup.{}.html", now))
}

async fn update_wiki(config: Config, body: Body) -> Result<(), String> {
    let content = body::to_bytes(body)
        .await
        .map_err(|err| format!("failed to read request body: {}", err))?;

    let fname = gen_backup_filename(&config.backup_dir);

    fs::write(&fname, content).await.map_err(|err| {
        format!(
            "failed to write backup wiki file ({}): {}",
            fname.display(),
            err
        )
    })?;
    info!(
        "saved to backup file {}",
        &fname.file_name().unwrap().to_string_lossy()
    );

    fs::copy(&fname, &config.wiki_file).await.map_err(|err| {
        format!(
            "failed to overwrite wiki file ({}): {}",
            config.wiki_file, err
        )
    })?;
    info!("wiki updated");

    Ok(())
}

async fn handle(config: Config, req: Request<Body>) -> HyperResult<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, _) => match fs::read_to_string(config.wiki_file).await {
            Ok(wiki) => Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(wiki)),
            Err(err) => {
                error!("couldn't read wiki file: {}", err);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("couldn't read wiki file: {}", err)))
            }
        },

        (&Method::PUT, _) => match update_wiki(config, req.into_body()).await {
            Ok(()) => Response::builder().body(Body::empty()),
            Err(err) => {
                error!("couldn't update wiki: {}", err);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(err))
            }
        },

        // This is how we inform TiddlyWiki that this server supports
        // PUT saving.
        (&Method::OPTIONS, _) => Response::builder()
            .header("dav", "tw5/put")
            .body(Body::empty()),

        _ => Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("Content-Type", "text/plain")
            .body(Body::empty()),
    }
}

#[tokio::main]
async fn main() {
    let config = Config::parse();

    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    fs::create_dir_all(&config.backup_dir)
        .await
        .map_err(|err| {
            error!("couldn't create backups directory: {}", err);
            process::exit(1);
        })
        .unwrap();

    let backup_fname = gen_backup_filename(&config.backup_dir);
    match fs::copy(&config.wiki_file, &backup_fname).await {
        Err(err) => {
            error!(
                "failed to back up wiki file to {}: {}",
                &backup_fname.display(),
                err
            );
            process::exit(1);
        }

        Ok(_) => (),
    };

    let address = IpAddr::from_str(&config.address)
        .map_err(|_| {
            error!("invalid IP address: {}", config.address);
            process::exit(1);
        })
        .unwrap();

    let server =
        Server::bind(&SocketAddr::from((address, config.port))).serve(make_service_fn(|_| {
            let config = config.clone();
            async { Ok::<_, Infallible>(service_fn(move |req| handle(config.clone(), req))) }
        }));

    info!("started server on {}:{}", address, config.port);

    tokio::spawn(tokio::signal::ctrl_c().map(|_| {
        info!("shutting down");
        process::exit(0);
    }));

    if let Err(e) = server.await {
        error!("server error: {}", e)
    }
}
