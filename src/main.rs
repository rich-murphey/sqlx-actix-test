use actix_web::*;
use anyhow::Context;
use log::*;
use sqlx::postgres::*;
use std::env;

mod junk;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    // setup database connection pool
    let db_url = env::var("DATABASE_URL").context("DATABASE_URL")?;
    let options: PgConnectOptions = db_url.parse().context(db_url.clone())?;
    let max_conn: u32 = match std::env::var("MAX_CONN") {
        Ok(s) => s.parse().context(s)?,
        _ => 90, // default postgres conn limit is 100
    };
    let pool = PgPoolOptions::new()
        .max_connections(max_conn)
        // NB: setting test_before_acquire(true) eliminates the protocol errors.
        // .test_before_acquire(false)
        .connect_with(options)
        .await
        .context(db_url.clone())?;
    junk::setup_db(&pool).await.context(db_url)?;

    // setup and run web server
    let addr = env::var("SOCKETADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    info!("this web server is listening at http://{}", &addr);
    HttpServer::new(move || {
        actix_web::App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .configure(junk::service)
            .default_service(web::route().to(HttpResponse::NotFound))
    })
    .bind(&addr)
    .context(addr.clone())?
    .run()
    .await
    .context(addr)?;

    Ok(())
}
