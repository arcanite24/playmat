//! playmat-swarm serve [--db swarm.db] [--listen 0.0.0.0:17190] --public-url URL [--store DIR]
//!     runs the coordinator. Without `--store`, results and networks go to Cloudflare R2: SWARM_R2_ACCOUNT,
//!     SWARM_R2_BUCKET, SWARM_R2_KEY_ID and SWARM_R2_SECRET. With it, to that folder, served by the coordinator (LAN).
//!     Options can also be set as SWARM_<NAME>: SWARM_DB, SWARM_LISTEN, SWARM_PUBLIC_URL, SWARM_STORE
//! playmat-swarm token --db swarm.db --role worker|trainer|admin --name NAME
//!     makes a token and prints it once; the database keeps only its hash

use playmat_swarm::store::{Local, Store, R2};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::Role;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args: Vec<String> = std::env::args().collect();
    // every option can also come from the environment (the Docker image): --public-url is SWARM_PUBLIC_URL
    let opt = |name: &str| {
        args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned().or_else(|| {
            std::env::var(format!("SWARM_{}", name.trim_start_matches('-').replace('-', "_").to_uppercase()))
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
    };
    let db = opt("--db").unwrap_or_else(|| "swarm.db".into());
    let base = opt("--public-url").unwrap_or_else(|| "http://localhost:17190".into());
    let store = match opt("--store") {
        Some(dir) => Store::Local(Local::new(PathBuf::from(dir), base.clone())),
        None if args.get(1).map(String::as_str) == Some("serve") => {
            // an empty setting (mana.env.example's blanks) counts as unset
            let env = |k: &str| {
                std::env::var(k)
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .ok_or_else(|| anyhow::anyhow!("{k} is not set (or set SWARM_STORE / pass --store DIR)"))
            };
            Store::R2(R2::new(
                &env("SWARM_R2_ACCOUNT")?,
                &env("SWARM_R2_BUCKET")?,
                &env("SWARM_R2_KEY_ID")?,
                &env("SWARM_R2_SECRET")?,
            ))
        }
        None => Store::Local(Local::new(PathBuf::from("store"), base.clone())),
    };
    let swarm = Swarm::open(&db, store, &base).await?;
    match args.get(1).map(String::as_str) {
        Some("serve") => {
            let listen = opt("--listen").unwrap_or_else(|| "0.0.0.0:17190".into());
            let listener = tokio::net::TcpListener::bind(&listen).await?;
            tracing::info!("Mana@Home coordinator on {listen}, public at {base}");
            playmat_swarm::serve(listener, swarm).await?;
        }
        Some("token") => {
            let role = opt("--role")
                .and_then(|r| Role::parse(&r))
                .ok_or_else(|| anyhow::anyhow!("--role worker|trainer|admin"))?;
            let name = opt("--name").ok_or_else(|| anyhow::anyhow!("--name NAME"))?;
            let t = swarm.new_token(role, &name).await?;
            println!("{}", t.token);
        }
        _ => anyhow::bail!("usage: playmat-swarm serve|token (see the source's header)"),
    }
    Ok(())
}
